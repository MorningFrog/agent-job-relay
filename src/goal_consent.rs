//! Scoped pause consent is independent of execution and native pause acknowledgement.
use crate::{
    core::*,
    goal_control::{Controller, Goal},
    storage::Store,
};
use anyhow::{Context, Result, ensure};
use rmcp::schemars;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const QUESTION_SECONDS: u64 = 300;
const PENDING_MS: i64 = 600_000;
const BOUNDED_APPROVAL_MS: i64 = 86_400_000;

#[derive(Clone, Default)]
pub struct Options {
    pub request_id: Option<String>,
    pub label: Option<String>,
    pub job_limit: Option<u32>,
    /// Cooperative assertion of an actual user request, never plugin text.
    pub user_authorized: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, rmcp::schemars::JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    #[default]
    Ask,
    /// Assert only an actual affirmative user reply to the scoped question.
    Allow,
    Decline,
    Revoke,
}

#[derive(Clone)]
pub struct Consent {
    pub id: String,
    pub session_key: String,
    pub controller: Controller,
    pub expected: Goal,
    snapshot: String,
    pub cwd: PathBuf,
    pub label: String,
    // The existing non-null column stores zero for new goal-lifetime consent.
    job_limit: u32,
    pub used: u32,
    pub status: String,
    pub revision: i64,
    pub expires: i64,
    token: Option<String>,
    pub source: String,
    /// Latest successful observation, never persisted as lifecycle authority.
    pub goal_status: Option<String>,
}

impl Consent {
    fn job_limit(&self) -> Option<u32> {
        (self.job_limit != 0).then_some(self.job_limit)
    }

    fn jobs_remaining(&self) -> Option<u32> {
        self.job_limit()
            .map(|limit| limit.saturating_sub(self.used))
    }

    pub(crate) fn question(&self) -> String {
        match self.job_limit() {
            Some(limit) => format!("允许本次请求的后台任务暂停目标，完成后继续吗？（最多 {limit} 项，授权有效期 24 小时）"),
            None => "允许同一目标下本次请求的后续后台任务暂停目标，完成后继续吗？批准后可沿用，直到目标结束或你撤回授权。".into(),
        }
    }
}

fn read(db: &Connection, id: &str) -> Result<Consent> {
    db.query_row("SELECT id,session_key,controller,expected,session_snapshot,cwd,label,job_limit,used,status,revision,expires,question_token,source FROM goal_consents WHERE id=?",[id],|r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,u32>(7)?,r.get::<_,u32>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?,r.get::<_,i64>(11)?,r.get::<_,Option<String>>(12)?,r.get::<_,String>(13)?)))
        .map_err(Into::into).and_then(|r|Ok(Consent{id:r.0,session_key:r.1,controller:serde_json::from_str(&r.2)?,expected:serde_json::from_str(&r.3)?,snapshot:r.4,cwd:r.5.into(),label:r.6,job_limit:r.7,used:r.8,status:r.9,revision:r.10,expires:r.11,token:r.12,source:r.13,goal_status:None}))
}
fn audit(db: &Connection, c: &Consent, source: &str) -> Result<()> {
    db.execute(
        "INSERT INTO goal_consent_audits(request_id,revision,at,status,source) VALUES(?,?,?,?,?)",
        params![c.id, c.revision, now(), c.status, source],
    )?;
    Ok(())
}
fn snapshot(s: &crate::session::AgentSession) -> Result<String> {
    Ok(serde_json::to_string(
        &json!({"registration":s.registration,"endpoint_identity":s.endpoint_identity}),
    )?)
}
fn owner(db: &Connection, c: &Consent, session: &str) -> Result<()> {
    ensure!(
        c.session_key == session,
        "pause consent belongs to another caller registration; it cannot be moved"
    );
    let actual = crate::session::read_session(db, session)?;
    ensure!(
        actual.connected && actual.expires_at > now() && snapshot(&actual)? == c.snapshot,
        "pause consent caller/endpoint changed; obtain a new current-terminal request"
    );
    Ok(())
}
fn close(db: &Connection, c: &Consent, status: &str, source: &str) -> Result<()> {
    db.execute("UPDATE goal_consents SET status=?,revision=revision+1,updated=?,source=? WHERE id=? AND revision=?",params![status,now(),source,c.id,c.revision])?;
    let updated = read(db, &c.id)?;
    audit(db, &updated, source)?;
    if matches!(status, "revoked" | "invalidated") {
        crate::goal_wait::cancel_for_consent(db, &c.id, source)?;
    }
    Ok(())
}
fn expire(db: &Connection, c: &Consent) -> Result<Consent> {
    if c.expires <= now() && matches!(c.status.as_str(), "pending" | "asking" | "approved") {
        close(
            db,
            c,
            "expired",
            "scope expired; no automatic re-prompt or submission",
        )?;
        return read(db, &c.id);
    }
    Ok(c.clone())
}
pub fn view(c: &Consent) -> Value {
    let remaining = c.jobs_remaining();
    let state = if c.status == "approved" && remaining == Some(0) {
        "exhausted"
    } else {
        &c.status
    };
    let goal_inactive = c.goal_status.as_deref().is_some_and(|s| s != "active");
    let mut value = json!({"request_id":c.id,"session_key":c.session_key,"status":state,"revision":c.revision,"label":c.label,"job_limit":c.job_limit(),"jobs_admitted":c.used,"jobs_remaining":remaining,"expires_at":(c.expires!=i64::MAX).then_some(c.expires),"question":c.question(),"choices":["允许","暂不运行"],"question_pending":c.status=="asking","authorization_source":c.source,"next_step":match state {
        "pending" | "asking" | "approved" if goal_inactive => "The original goal is inactive and this request is retained. Do not submit, re-prompt, mark blocked or automatically overwrite the host status. If a recorded wait is still waiting, let it finish. Otherwise the user can resume the original goal through host controls, then reconnect with this same request_id; approved consent needs no new pause question.",
        "pending"=>"Use job_goal_consent to request native confirmation, or ask one short host question. Only an actual user answer may be recorded with decision=allow/decline and this revision. Do not submit work or mark the goal blocked.",
        "asking"=>"The original question is pending. Wait for that answer; do not ask again, poll or submit work.",
        "approved"=>"Reconnect from the actual terminal with --request-id and this request_id. Reuse the approval for later rounds of this request on the original goal. Preserve the returned goal binding.",
        "exhausted"=>"This bounded approval has used its explicit AgentJobRelay background-job allowance. Follow any recorded handoff and inspect the original Job/Run results. Continue authorized foreground work in the active goal. The Relay allowance alone does not block the goal. A new approval is needed to change the scope of the original permission.",
        _=>"This pause-consent request is closed for new AgentJobRelay background jobs. Retain the original Job/Run/wait IDs and respect any user decision. Continue authorized foreground work if the goal is active. Do not mark the goal blocked solely because this Relay request is closed. Do not replay jobs or re-prompt automatically."
    }});
    if goal_inactive {
        value["goal_status"] = json!(c.goal_status);
    }
    value
}
fn view_db(db: &Connection, c: &Consent) -> Result<Value> {
    let mut value = view(c);
    let mut q=db.prepare("SELECT job_id,run_id,wait_id,status FROM (SELECT m.rowid AS seq,m.job_id,m.run_id,m.wait_id,w.status FROM goal_wait_jobs m JOIN goal_waits w ON w.id=m.wait_id WHERE w.consent_id=? ORDER BY m.rowid DESC LIMIT 16) ORDER BY seq")?;
    value["jobs"]=json!(q.query_map([&c.id],|r|Ok(json!({"job_id":r.get::<_,String>(0)?,"run_id":r.get::<_,String>(1)?,"goal_wait_id":r.get::<_,String>(2)?,"handoff_status":r.get::<_,String>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?);
    value["waits"] = json!(crate::goal_wait::for_request(db, &c.id)?);
    Ok(value)
}

impl Store {
    pub async fn clear_obsolete_goal_consents(&self, session: String) -> Result<()> {
        let key = session.clone();
        let ids=self.read(move|db|{let mut q=db.prepare("SELECT id FROM goal_consents WHERE session_key=? AND status IN ('pending','asking','approved') LIMIT 100")?;Ok(q.query_map([key],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)}).await?;
        for id in ids {
            self.goal_consent_record(id, session.clone()).await?;
        }
        Ok(())
    }
    pub async fn goal_consent_view(&self, id: String, session: String) -> Result<Value> {
        let c = self.goal_consent_record(id, session).await?;
        self.read(move |db| view_db(db, &c)).await
    }
    pub async fn prepare_goal_consent(
        &self,
        session_key: String,
        controller: Controller,
        expected: Goal,
        cwd: PathBuf,
        options: Options,
    ) -> Result<Value> {
        controller.validate_async().await?;
        let current = controller
            .read()
            .await?
            .context("goal removed before consent preparation")?;
        ensure!(
            expected.same_identity(&current) && current.status == "active",
            "goal changed before consent preparation"
        );
        let s = self.session(session_key.clone()).await?;
        ensure!(
            s.registration.adapter == crate::session::Adapter::DesktopExperimental
                && s.registration.session_id == controller.thread_id
                && s.registration.caller_thread_id.as_ref() == Some(&controller.thread_id),
            "consent requires the actual Desktop caller"
        );
        crate::adapters::probe(&s).await?;
        let snap = snapshot(&s)?;
        self.call(move|db|{
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let label=options.label.clone().unwrap_or_else(||"本次后台任务".into());
            ensure!(!label.trim().is_empty() && label.len()<=512 && !label.chars().any(char::is_control),"request label must be 1..512 bytes without control characters");
            ensure!(options.job_limit.is_none_or(|limit| (1..=16).contains(&limit)),"explicit request job limit must be 1..16");
            let limit=options.job_limit.unwrap_or(0);
            let actual=crate::session::read_session(&tx,&session_key)?;
            ensure!(actual.connected && actual.expires_at>now() && snapshot(&actual)?==snap,"caller changed before consent preparation");
            let old=if let Some(id)=&options.request_id {validate_id(id)?;Some(id.clone())}else{
                let ids={let mut q=tx.prepare("SELECT id FROM goal_consents WHERE session_key=? AND status IN ('pending','asking') AND expires>? AND cwd=? AND label=? AND job_limit=? AND session_snapshot=? LIMIT 100")?;q.query_map(params![session_key,now(),cwd.to_string_lossy(),label,limit,snap],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?};
                ids.into_iter().find(|id|read(&tx,id).is_ok_and(|c|c.expected.same_identity(&current)))
            };
            let id=if let Some(id)=old {
                let c=read(&tx,&id)?;owner(&tx,&c,&session_key)?;
                ensure!(c.cwd==cwd && options.label.as_ref().is_none_or(|v|v==&c.label) && options.job_limit.is_none_or(|v|v==c.job_limit),"request scope cannot be changed after preparation");
                if !c.expected.same_identity(&current) || serde_json::to_value(&c.controller)?!=serde_json::to_value(&controller)? {close(&tx,&c,"invalidated","original goal/controller changed")?;}
                id
            }else{
                let count:i64=tx.query_row("SELECT count(*) FROM goal_consents",[],|r|r.get(0))?;ensure!(count<10000,"pause consent history quota reached");
                let id=id();let t=now();
                tx.execute("INSERT INTO goal_consents(id,session_key,controller,expected,session_snapshot,cwd,label,job_limit,status,created,updated,expires) VALUES(?,?,?,?,?,?,?,?,'pending',?,?,?)",params![id,session_key,serde_json::to_string(&controller)?,serde_json::to_string(&current)?,snap,cwd.to_string_lossy(),label,limit,t,t,t+PENDING_MS])?;
                audit(&tx,&read(&tx,&id)?,"current-terminal request; no goal mutation or work admitted")?;id
            };
            let c=expire(&tx,&read(&tx,&id)?)?;
            if options.user_authorized && c.status=="pending" {approve(&tx,&c,"terminal assertion of actual user consent")?;}
            let result=view_db(&tx,&read(&tx,&id)?)?;tx.commit()?;Ok(result)
        }).await
    }

    async fn local_goal_consent_record(&self, id: String, session_key: String) -> Result<Consent> {
        validate_id(&id)?;
        validate_id(&session_key)?;
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let c = read(&tx, &id)?;
            owner(&tx, &c, &session_key)?;
            let c = expire(&tx, &c)?;
            tx.commit()?;
            Ok(c)
        })
        .await
    }

    pub async fn goal_consent_record(&self, id: String, session_key: String) -> Result<Consent> {
        let mut c = self.local_goal_consent_record(id, session_key).await?;
        if matches!(c.status.as_str(), "pending" | "asking" | "approved") {
            // Transport/controller failure is not evidence of goal deletion.
            // Fail closed for this call without destroying consent or waits.
            let actual = c.controller.read().await?;
            if actual.as_ref().is_none_or(|g| {
                !c.expected.same_identity(g)
                    || matches!(g.status.as_str(), "complete" | "completed")
            }) {
                let id = c.id.clone();
                let revision = c.revision;
                self.call(move |db| {
                    let tx =
                        db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                    let c = read(&tx, &id)?;
                    if c.revision == revision {
                        close(
                            &tx,
                            &c,
                            "invalidated",
                            "original goal removed, replaced or completed",
                        )?;
                    }
                    let result = read(&tx, &id)?;
                    tx.commit()?;
                    Ok(result)
                })
                .await
            } else {
                c.goal_status = actual.map(|g| g.status);
                Ok(c)
            }
        } else {
            Ok(c)
        }
    }

    pub async fn decide_goal_consent(
        &self,
        id: String,
        session_key: String,
        revision: i64,
        decision: Decision,
        source: &'static str,
    ) -> Result<Value> {
        // Withdrawing consent must remain possible while the host is offline.
        let c = if matches!(decision, Decision::Decline | Decision::Revoke) {
            self.local_goal_consent_record(id.clone(), session_key.clone())
                .await?
        } else {
            self.goal_consent_record(id.clone(), session_key.clone())
                .await?
        };
        if matches!(c.status.as_str(), "expired" | "invalidated" | "revoked") {
            return Ok(view(&c));
        }
        ensure!(
            !matches!(decision, Decision::Ask),
            "asking is handled by the host tool, not a consent assertion"
        );
        if matches!(decision, Decision::Allow) {
            ensure!(
                c.controller
                    .read()
                    .await?
                    .is_some_and(|g| c.expected.same_identity(&g) && g.status == "active"),
                "the original goal must still be active before approval"
            );
        }
        self.call(move|db|{let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;let c=expire(&tx,&read(&tx,&id)?)?;owner(&tx,&c,&session_key)?;
            let wanted=match decision {Decision::Allow=>"approved",Decision::Decline=>"declined",Decision::Revoke=>"revoked",Decision::Ask=>unreachable!()};
            if c.status==wanted && revision.checked_add(1)==Some(c.revision) {let v=view_db(&tx,&c)?;tx.commit()?;return Ok(v);}
            ensure!(c.revision==revision,"consent revision changed; inspect the existing request instead of replaying an answer");
            ensure!(if wanted=="revoked"{matches!(c.status.as_str(),"pending"|"asking"|"approved")}else{c.status=="pending" || (c.status=="asking" && c.token.is_none())},"consent is already resolved, expired or has a pending native question");
            if wanted=="approved" {approve(&tx,&c,source)?;}else{close(&tx,&c,wanted,source)?;}
            let v=view_db(&tx,&read(&tx,&id)?)?;tx.commit()?;Ok(v)
        }).await
    }

    pub async fn claim_goal_question(
        &self,
        id: String,
        session_key: String,
        revision: i64,
        native: bool,
    ) -> Result<Consent> {
        let observed = self
            .goal_consent_record(id.clone(), session_key.clone())
            .await?;
        ensure!(
            observed.goal_status.as_deref() == Some("active"),
            "the original goal must be active before asking for pause consent; existing consent was retained"
        );
        self.call(move|db|{let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;let c=expire(&tx,&read(&tx,&id)?)?;owner(&tx,&c,&session_key)?;
            ensure!(c.status=="pending"&&c.revision==revision,"question already pending/resolved; do not create another prompt");
            tx.execute("UPDATE goal_consents SET status='asking',revision=revision+1,updated=?,expires=?,question_token=? WHERE id=? AND revision=?",params![now(),now()+QUESTION_SECONDS as i64*1000,if native {Some(crate::core::id())}else{None},c.id,c.revision])?;
            let c=read(&tx,&c.id)?;audit(&tx,&c,"native question claimed; no user answer yet")?;tx.commit()?;Ok(c)
        }).await
    }
    pub fn question_token(c: &Consent) -> Result<String> {
        c.token.clone().context("missing native question token")
    }
    pub async fn answer_goal_question(
        &self,
        id: String,
        session_key: String,
        token: String,
        allow: bool,
    ) -> Result<Value> {
        let c = if allow {
            self.goal_consent_record(id.clone(), session_key.clone())
                .await?
        } else {
            self.local_goal_consent_record(id.clone(), session_key.clone())
                .await?
        };
        if matches!(c.status.as_str(), "expired" | "invalidated" | "revoked") {
            return self.goal_consent_view(id, session_key).await;
        }
        if allow {
            ensure!(
                c.controller
                    .read()
                    .await?
                    .is_some_and(|g| c.expected.same_identity(&g) && g.status == "active"),
                "goal changed while the question was pending"
            );
        }
        self.call(move |db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let c = expire(&tx, &read(&tx, &id)?)?;
            owner(&tx, &c, &session_key)?;
            if c.status == if allow { "approved" } else { "declined" }
                && c.token.as_ref() == Some(&token)
            {
                let v = view_db(&tx, &c)?;
                tx.commit()?;
                return Ok(v);
            }
            ensure!(
                c.status == "asking" && c.token.as_ref() == Some(&token),
                "native answer is stale, expired or belongs to another question"
            );
            if allow {
                approve(&tx, &c, "native user confirmation")?;
            } else {
                close(
                    &tx,
                    &c,
                    "declined",
                    "native decline/cancel or unavailable answer",
                )?;
            }
            let v = view_db(&tx, &read(&tx, &id)?)?;
            tx.commit()?;
            Ok(v)
        })
        .await
    }
}
fn approve(db: &Connection, c: &Consent, source: &str) -> Result<()> {
    let t = now();
    let expires = if c.job_limit().is_some() {
        t + BOUNDED_APPROVAL_MS
    } else {
        i64::MAX
    };
    db.execute("UPDATE goal_consents SET status='approved',revision=revision+1,updated=?,expires=?,source=? WHERE id=? AND revision=?",params![t,expires,source,c.id,c.revision])?;
    audit(db, &read(db, &c.id)?, source)
}

pub(crate) fn check_scope(
    db: &Connection,
    id: &str,
    session: &str,
    goal: &Goal,
    controller: &Controller,
    cwd: Option<&std::path::Path>,
) -> Result<()> {
    let c = read(db, id)?;
    owner(db, &c, session)?;
    ensure!(
        c.status == "approved" && c.expires > now() && c.jobs_remaining() != Some(0),
        "new AgentJobRelay background jobs require an active pause approval covering this submission"
    );
    ensure!(
        c.expected.same_identity(goal)
            && serde_json::to_value(&c.controller)? == serde_json::to_value(controller)?
            && cwd.is_none_or(|p| p == c.cwd),
        "pause consent cannot move to another goal, controller or workspace"
    );
    Ok(())
}
pub(crate) fn consume(
    db: &Connection,
    id: &str,
    session: &str,
    goal: &Goal,
    controller: &Controller,
    cwd: &std::path::Path,
) -> Result<()> {
    check_scope(db, id, session, goal, controller, Some(cwd))?;
    db.execute(
        "UPDATE goal_consents SET used=used+1,updated=? WHERE id=?",
        params![now(), id],
    )?;
    audit(
        db,
        &read(db, id)?,
        "one original Job/Run recorded in the consent history transactionally",
    )
}
pub(crate) fn restore_fence(db: &Connection) -> Result<()> {
    let ids = {
        let mut q = db.prepare(
            "SELECT id FROM goal_consents WHERE status IN ('pending','asking','approved')",
        )?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for id in ids {
        close(
            db,
            &read(db, &id)?,
            "invalidated",
            "backup restore cannot transfer pause consent",
        )?;
    }
    Ok(())
}
