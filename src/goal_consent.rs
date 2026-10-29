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
const APPROVED_MS: i64 = 86_400_000;

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
    pub job_limit: u32,
    pub used: u32,
    pub status: String,
    pub revision: i64,
    pub expires: i64,
    token: Option<String>,
    pub source: String,
}

fn read(db: &Connection, id: &str) -> Result<Consent> {
    db.query_row("SELECT id,session_key,controller,expected,session_snapshot,cwd,label,job_limit,used,status,revision,expires,question_token,source FROM goal_consents WHERE id=?",[id],|r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,u32>(7)?,r.get::<_,u32>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?,r.get::<_,i64>(11)?,r.get::<_,Option<String>>(12)?,r.get::<_,String>(13)?)))
        .map_err(Into::into).and_then(|r|Ok(Consent{id:r.0,session_key:r.1,controller:serde_json::from_str(&r.2)?,expected:serde_json::from_str(&r.3)?,snapshot:r.4,cwd:r.5.into(),label:r.6,job_limit:r.7,used:r.8,status:r.9,revision:r.10,expires:r.11,token:r.12,source:r.13}))
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
    let state = if c.status == "approved" && c.used >= c.job_limit {
        "exhausted"
    } else {
        &c.status
    };
    json!({"request_id":c.id,"session_key":c.session_key,"status":state,"revision":c.revision,"label":c.label,"job_limit":c.job_limit,"jobs_admitted":c.used,"jobs_remaining":c.job_limit.saturating_sub(c.used),"expires_at":c.expires,"question":"允许本次后台任务暂停目标，完成后继续吗？","choices":["允许","暂不运行"],"question_pending":c.status=="asking","authorization_source":c.source,"next_step":match state {
        "pending"=>"Use job_goal_consent to request native confirmation, or ask one short host question. Only an actual user answer may be recorded with decision=allow/decline and this revision. Do not submit work or mark the goal blocked.",
        "asking"=>"The original question is pending. Wait for that answer; do not ask again, poll or submit work.",
        "approved"=>"Reconnect from the actual terminal with --request-id and this request_id; preserve the returned goal binding. Reuse this consent only for the remaining rounds of this request and original goal.",
        _=>"This request cannot admit further work. Existing jobs were not replayed or stopped. Do not re-prompt automatically; follow the user's next instruction."
    }})
}
fn view_db(db: &Connection, c: &Consent) -> Result<Value> {
    let mut value = view(c);
    let mut q=db.prepare("SELECT m.job_id,m.run_id,w.id,w.status FROM goal_wait_jobs m JOIN goal_waits w ON w.id=m.wait_id WHERE w.consent_id=? ORDER BY m.rowid LIMIT 16")?;
    value["jobs"]=json!(q.query_map([&c.id],|r|Ok(json!({"job_id":r.get::<_,String>(0)?,"run_id":r.get::<_,String>(1)?,"goal_wait_id":r.get::<_,String>(2)?,"handoff_status":r.get::<_,String>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?);
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
            let limit=options.job_limit.unwrap_or(1);ensure!((1..=16).contains(&limit),"request job limit must be 1..16");
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

    pub async fn goal_consent_record(&self, id: String, session_key: String) -> Result<Consent> {
        validate_id(&id)?;
        validate_id(&session_key)?;
        let c = self
            .call(move |db| {
                let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                let c = read(&tx, &id)?;
                owner(&tx, &c, &session_key)?;
                let c = expire(&tx, &c)?;
                tx.commit()?;
                Ok(c)
            })
            .await?;
        if matches!(c.status.as_str(), "pending" | "asking" | "approved") {
            let actual = c.controller.read().await.ok().flatten();
            if actual.as_ref().is_none_or(|g| {
                !c.expected.same_identity(g) || !matches!(g.status.as_str(), "active" | "paused")
            }) {
                let id = c.id.clone();
                self.call(move |db| {
                    let tx =
                        db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                    let c = read(&tx, &id)?;
                    close(
                        &tx,
                        &c,
                        "invalidated",
                        "original goal removed/replaced or controller unavailable",
                    )?;
                    let result = read(&tx, &id)?;
                    tx.commit()?;
                    Ok(result)
                })
                .await
            } else {
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
        let c = self
            .goal_consent_record(id.clone(), session_key.clone())
            .await?;
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
        self.goal_consent_record(id.clone(), session_key.clone())
            .await?;
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
        let c = self
            .goal_consent_record(id.clone(), session_key.clone())
            .await?;
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
    db.execute("UPDATE goal_consents SET status='approved',revision=revision+1,updated=?,expires=?,source=? WHERE id=? AND revision=?",params![now(),now()+APPROVED_MS,source,c.id,c.revision])?;
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
        c.status == "approved" && c.expires > now() && c.used < c.job_limit,
        "request consent is pending, closed, expired or exhausted"
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
        "one original Job/Run admitted; consent budget consumed transactionally",
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
