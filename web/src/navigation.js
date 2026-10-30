// Consume a fresh private-IPC URL on initial load and same-document navigation.
export function consumePanelNavigation(browser = window) {
  const {location, history, sessionStorage} = browser;
  const params = new URLSearchParams(location.hash.slice(1));
  const incoming = params.get('token');
  if (incoming) sessionStorage.setItem('ajr-token', incoming);
  if (params.has('token') || params.has('job')) {
    history.replaceState(null, '', location.pathname + location.search);
  }
  return {jobId: params.get('job'), token: sessionStorage.getItem('ajr-token') || ''};
}

export function subscribePanelNavigation(receive, browser = window) {
  const navigate = () => receive(consumePanelNavigation(browser));
  browser.addEventListener('hashchange', navigate);
  return () => browser.removeEventListener('hashchange', navigate);
}
