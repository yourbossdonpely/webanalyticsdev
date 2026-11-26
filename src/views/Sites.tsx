// Web Analytics Dev by Don PELY — tracked sites + tracker install view.
import { api, errMsg } from '../lib/api';
import type { Site } from '../lib/types';

export default function Sites({
  sites,
  onSites,
  onOpenSite,
  toast,
  openModal,
  closeModal,
}: {
  sites: Site[];
  onSites: (sites: Site[]) => void;
  onOpenSite: (domain: string) => void;
  toast: (msg: string, isError?: boolean) => void;
  openModal: (content: React.ReactNode) => void;
  closeModal: () => void;
}) {
  const showSnippet = (domain: string) => {
    api
      .snippet(domain)
      .then((snippet) => {
        openModal(
          <>
            <h2>Install the tracker</h2>
            <p style={{ color: 'var(--muted)' }}>
              Paste this just before <code>&lt;/head&gt;</code> on <strong>{domain}</strong>. It is ~4 KB,
              cookieless, and also tracks SPA page changes. Custom events: <code>wadev("Signup")</code> or{' '}
              <code>class="wadev-event-Signup"</code>.
            </p>
            <pre>{snippet}</pre>
            <div className="modal-actions">
              <button className="btn ghost" onClick={closeModal}>
                Close
              </button>
              <button
                className="btn"
                onClick={() => {
                  navigator.clipboard
                    .writeText(snippet)
                    .then(() => toast('Snippet copied.'))
                    .catch(() => toast('Copy failed — select the text manually.', true));
                }}
              >
                Copy snippet
              </button>
            </div>
          </>,
        );
      })
      .catch((e) => toast(`Get snippet: ${errMsg(e)}`, true));
  };

  const showAdd = () => {
    let domain = '';
    let name = '';
    openModal(
      <>
        <h2>Add a site</h2>
        <div className="form-row">
          <label>
            Domain
            <input placeholder="example.com" onChange={(e) => (domain = e.target.value)} />
          </label>
        </div>
        <div className="form-row">
          <label>
            Display name (optional)
            <input placeholder="My website" onChange={(e) => (name = e.target.value)} />
          </label>
        </div>
        <div className="modal-actions">
          <button className="btn ghost" onClick={closeModal}>
            Cancel
          </button>
          <button
            className="btn"
            onClick={() => {
              api
                .addSite(domain, name)
                .then((site) => {
                  onSites([...sites, site]);
                  closeModal();
                  showSnippet(site.domain);
                })
                .catch((e) => toast(`Add site: ${errMsg(e)}`, true));
            }}
          >
            Add site
          </button>
        </div>
      </>,
    );
  };

  return (
    <>
      <div className="view-title">
        <h2>🌐 Tracked sites</h2>
        <button className="btn" onClick={showAdd}>
          ＋ Add site
        </button>
      </div>
      <div className="card">
        <h3>Your sites</h3>
        {sites.length === 0 && (
          <p style={{ color: 'var(--muted)' }}>
            No sites yet. Add your first site, then paste the tracker snippet into its HTML.
          </p>
        )}
        {sites.map((s) => (
          <div className="mon" key={s.id}>
            <span aria-hidden="true">🌐</span>
            <div className="mon-info">
              <strong>{s.name}</strong>
              <small>
                {s.domain} · added {new Date(s.createdAt).toLocaleDateString()}
              </small>
            </div>
            <div style={{ display: 'flex', gap: 6 }}>
              <button className="btn xs ghost" onClick={() => showSnippet(s.domain)}>
                Snippet
              </button>
              <button className="btn xs ghost" onClick={() => onOpenSite(s.domain)}>
                Open
              </button>
              <button
                className="btn xs danger"
                onClick={() => {
                  if (confirm(`Stop tracking ${s.domain}? Past events stay on disk.`)) {
                    api
                      .removeSite(s.id)
                      .then(onSites)
                      .catch((e) => toast(`Remove site: ${errMsg(e)}`, true));
                  }
                }}
              >
                ✕
              </button>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
