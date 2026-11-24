const me = Math.random().toString(36).slice(2)
const lang = (navigator.language || 'en').toLowerCase()
const fr = lang.startsWith('fr')

const T = fr
  ? {
      title: 'Mails',
      search: 'Chercher (to:client@exemple.test subject:commande)',
      refresh: 'Actualiser',
      clear: 'Tout effacer',
      confirm: 'Confirmer l’effacement',
      loading: 'Chargement…',
      empty: 'Aucun mail. Les mails que le site envoie en local arrivent ici.',
      back: '← Mails',
      from: 'De',
      to: 'À',
      links: 'Liens',
      html: 'Voir le HTML',
      text: 'Voir le texte',
      count: (n) => `${n} mail${n > 1 ? 's' : ''}`,
    }
  : {
      title: 'Mails',
      search: 'Search (to:client@example.test subject:order)',
      refresh: 'Refresh',
      clear: 'Delete all',
      confirm: 'Confirm deletion',
      loading: 'Loading…',
      empty: 'No mail. The mails the site sends locally come here.',
      back: '← Mails',
      from: 'From',
      to: 'To',
      links: 'Links',
      html: 'Show the HTML',
      text: 'Show the text',
      count: (n) => `${n} mail${n > 1 ? 's' : ''}`,
    }

const state = { list: null, url: '', total: 0, detail: null, opening: null, html: false, query: '', error: '', confirm: 0 }
const send = (type, extra = {}) => squadron.send({ ...extra, type, to: me, lang })

function h(tag, props = {}, ...children) {
  const el = document.createElement(tag)
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined || v === null || v === false) continue
    if (k.startsWith('on')) el.addEventListener(k.slice(2), v)
    else if (k in el) el[k] = v
    else el.setAttribute(k, v)
  }
  el.append(...children.flat(Infinity).filter((c) => c !== null && c !== undefined && c !== false))
  return el
}

const when = (iso) => (Date.parse(iso) ? new Date(iso).toLocaleString(lang, { dateStyle: 'short', timeStyle: 'short' }) : '')

function load() {
  state.error = ''
  state.list = null
  render()
  send('list', { query: state.query })
}

function clearAll() {
  if (!state.confirm || Date.now() - state.confirm > 4000) {
    state.confirm = Date.now()
    return render()
  }
  state.confirm = 0
  state.list = null
  render()
  send('clear')
}

function open(m) {
  state.detail = null
  state.html = false
  state.opening = m.id
  render()
  send('open', { id: m.id })
}

function listView() {
  const confirming = state.confirm && Date.now() - state.confirm <= 4000
  return [
    h(
      'div',
      { className: 'bar' },
      h('input', { type: 'search', placeholder: T.search, value: state.query, 'aria-label': T.search, oninput: (e) => { state.query = e.target.value }, onkeydown: (e) => e.key === 'Enter' && load() }),
      h('button', { onclick: load }, T.refresh),
      h('button', { className: confirming ? 'danger' : '', disabled: !state.list?.length, onclick: clearAll }, confirming ? T.confirm : T.clear),
    ),
    state.list === null
      ? h('p', { className: 'muted' }, T.loading)
      : !state.list.length
        ? h('p', { className: 'muted empty' }, T.empty)
        : h(
            'ul',
            { className: 'issues' },
            state.list.map((m) =>
              h(
                'li',
                {},
                h(
                  'button',
                  { className: 'issue mail', onclick: () => open(m) },
                  h('span', { className: 'key' }, when(m.created)),
                  h('span', { className: 'title' }, m.subject || '—'),
                  h('span', {}, !m.read && h('span', { className: 'tag ok' }, '•')),
                  h('span', { className: 'meta muted' }, `${m.from} → ${m.to}`),
                ),
              ),
            ),
          ),
  ]
}

function detailView() {
  const d = state.detail
  const back = h('button', { onclick: () => { state.detail = null; state.opening = null; render() } }, T.back)
  if (!d) return [h('div', { className: 'bar' }, back), h('p', { className: 'muted' }, T.loading)]
  return [
    h('div', { className: 'bar' }, back, h('span', { className: 'status' }), d.html && h('button', { onclick: () => { state.html = !state.html; render() } }, state.html ? T.text : T.html)),
    h('div', { className: 'detail' }, h('h2', {}, d.mail.subject || '—'), h('p', { className: 'muted' }, `${T.from} ${d.mail.from} · ${T.to} ${d.mail.to} · ${when(d.mail.created)}`)),
    state.html ? h('iframe', { sandbox: '', srcdoc: d.html, title: d.mail.subject }) : h('pre', {}, d.text || '—'),
    d.links.length > 0 && [h('h3', {}, T.links), h('ul', { className: 'links' }, d.links.map((l) => h('li', {}, l)))],
  ]
}

function render() {
  const status = state.list ? `${T.count(state.total)} · ${state.url}` : ''
  document.getElementById('app').replaceChildren(
    ...[h('header', { className: 'bar' }, h('strong', {}, T.title), h('span', { className: 'status muted', role: 'status' }, status)), state.error && h('p', { className: 'error' }, state.error), state.opening ? detailView() : listView()]
      .flat(Infinity)
      .filter(Boolean),
  )
}

squadron.on((msg) => {
  if (!msg || msg.to !== me) return
  if (msg.type === 'list') {
    state.list = msg.mails
    state.total = msg.total
    state.url = msg.url
  } else if (msg.type === 'detail') state.detail = msg.detail
  else if (msg.type === 'error') {
    state.error = msg.message
    state.opening = null
    if (state.list === null) state.list = []
  }
  render()
})

render()
send('ready')
