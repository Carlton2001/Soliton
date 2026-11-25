const byId = id => document.getElementById(id);
function savedTheme() {
  try {
    return localStorage.getItem('soliton-webdesign-theme') === 'light' ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}
const state = {
  view: 'albums', theme: savedTheme(), english: false, selected: null,
  query: '', artistQuery: '', genre: '', sort: 'artist', layout: 'grid', artistLayout: 'grid', artist: '', playlist: null,
  queue: [...library[0].tracks], position: 0, elapsed: 1, playing: false, volume: 50,
  shuffle: false, repeat: false,
};
const tr = (fr, en) => state.english ? en : fr;
const esc = value => String(value).replace(/[&<>"']/g, char =>
  ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const time = seconds => Math.floor(seconds / 60) + ':' + String(seconds % 60).padStart(2, '0');
const icon = name => '<span class="icon icon-' + name + '" aria-hidden="true"></span>';
function button(label, action, options = {}) {
  const { symbol, value = '', primary, pressed, current, showLabel = !symbol } = options;
  return '<button type="button" data-action="' + action + '" data-value="' + esc(value) +
    '" title="' + esc(label) + '" aria-label="' + esc(label) + '" class="' +
    [symbol && 'icon-button', primary && 'primary'].filter(Boolean).join(' ') + '"' +
    (pressed === undefined ? '' : ' aria-pressed="' + pressed + '"') +
    (current ? ' aria-current="page"' : '') + '>' +
    (symbol ? icon(symbol) : '') + (showLabel ? '<span>' + esc(label) + '</span>' : '') + '</button>';
}
const viewLabels = () => ({
  artists: tr('Artistes', 'Artists'), albums: 'Albums', playlists: 'Playlists',
  queue: tr('File de lecture', 'Play Queue'), settings: tr('Réglages', 'Settings'),
});
const sectionIcons = { artists: 'users', albums: 'disc-3', playlists: 'list', queue: 'list-end' };
const themes = () => ({ light: tr('Clair', 'Light'), dark: tr('Sombre', 'Dark') });
const audioFormat = 'FLAC · 44100 Hz · 16 bits';
const albumDuration = album => time(album.tracks.reduce((sum, track) => sum + track.seconds, 0));
const sortLabels = () => ({
  artist: tr('Tri : Artiste', 'Sort: Artist'),
  title: tr('Tri : Titre', 'Sort: Title'),
  year: tr('Tri : Année', 'Sort: Year'),
});
const progress = (value, max) => '--progress:' + (max ? 100 * value / max : 0) + '%';
const genreName = genre => genre === 'Electronic' ? tr('Électronique', 'Electronic') : genre;
const trackCount = tracks => tracks.length + ' ' + tr(tracks.length === 1 ? 'piste' : 'pistes', tracks.length === 1 ? 'track' : 'tracks');
const albumCount = albums => albums.length + (albums.length === 1 ? ' album' : ' albums');
const currentTrack = () => state.queue[state.position];
const isCurrent = track => track === currentTrack();
const cover = album => '<span class="cover" aria-hidden="true" style="--x:' +
  album.id % 3 * 50 + '%;--y:' + Math.floor(album.id / 3) * 50 + '%"></span>';
function themeButtons() {
  return Object.entries(themes()).map(([value, label]) => button(label, 'theme', { value, pressed: state.theme === value })).join('');
}
function renderNavigation() {
  byId('navigation').setAttribute('aria-label', tr('Navigation principale', 'Main navigation'));
  const labels = viewLabels();
  const settings = esc(labels.settings);
  byId('navigation').innerHTML = '<button type="button" class="rail-brand" data-action="navigate" data-value="settings"' +
    ' aria-label="' + settings + '" title="' + settings + '"' +
    (state.view === 'settings' ? ' aria-current="page"' : '') +
    '><img src="../assets/logos/blue.svg" alt=""></button>' +
    Object.entries(sectionIcons).map(([value, symbol]) => button(labels[value], 'navigate', {
      value, symbol, current: state.view === value, showLabel: true,
    })).join('') + button(tr('Mise à jour disponible', 'Update available'), 'update', { symbol: 'info' });
}
function option(value, label, current) {
  return '<option value="' + esc(value) + '"' + (value === current ? ' selected' : '') + '>' + esc(label) + '</option>';
}
function heading(title, toolbar = '') {
  return '<header class="heading"><h1>' + esc(title) + '</h1>' + toolbar + '</header>';
}
function filteredAlbums() {
  const query = state.query.trim().toLocaleLowerCase();
  const albums = library.filter(album => (!state.artist || state.artist === album.artist) &&
    (!state.genre || state.genre === album.genre) &&
    [album.title, album.artist, ...album.tracks.map(track => track.title)]
      .some(value => value.toLocaleLowerCase().includes(query)));
  if (state.sort === 'title') albums.sort((a, b) => a.title.localeCompare(b.title, document.documentElement.lang));
  if (state.sort === 'year') albums.sort((a, b) => b.year - a.year);
  if (state.sort === 'artist') albums.sort((a, b) => a.artist.localeCompare(b.artist) || a.title.localeCompare(b.title));
  return albums;
}
function renderCatalog() {
  byId('catalog').innerHTML = filteredAlbums().map(album =>
    '<button type="button" class="album" data-action="album" data-value="' + album.id +
    '" aria-pressed="' + (state.selected === album.id) + '">' + cover(album) +
    '<span class="item-copy"><strong>' + esc(album.title) + '</strong><small>' + esc(album.artist) +
    '</small></span><span class="album-meta"><small>' + album.year + ' · ' + genreName(album.genre) +
    '</small><small>FLAC · ' + albumDuration(album) + '</small></span></button>'
  ).join('') || '<p class="empty">' + tr('Aucun album trouvé.', 'No albums found.') + '</p>';
}
function albumView() {
  const search = tr('Rechercher un album, un artiste ou un titre', 'Search for an album, artist or track');
  const genre = tr('Tous les genres', 'All genres');
  const filters = tr('Filtres', 'Filters');
  return heading('Albums', '<div class="toolbar">' +
    button(tr('Tout lire', 'Play all'), 'play-results', { symbol: 'play', primary: true, showLabel: true }) +
    '<label class="search">' + icon('search') + '<input id="search" type="search" aria-label="' + search +
    '" placeholder="' + search + '" value="' + esc(state.query) + '"></label>' +
    '<label class="picker">' + icon('tag') + '<span class="picker-label" aria-hidden="true">' +
    (state.genre ? genreName(state.genre) : filters) + '</span><select id="genre" aria-label="' + genre + '">' + option('', filters, state.genre) +
    [...new Set(library.map(album => album.genre))].map(value => option(value, genreName(value), state.genre)).join('') + '</select>' + icon('chevron-down') + '</label>' +
    '<label class="picker">' + icon('arrow-up-down') + '<span class="picker-label" aria-hidden="true">' +
    sortLabels()[state.sort] + '</span><select id="sort" aria-label="' + tr('Trier les albums', 'Sort albums') + '">' +
    Object.entries(sortLabels()).map(([value, label]) => option(value, label, state.sort)).join('') +
    '</select>' + icon('chevron-down') + '</label>' + layoutButtons('layout', state.layout) +
    '</div>' + (state.artist ? '<div class="toolbar">' +
    button(state.artist + ' ×', 'clear-artist') + '</div>' : '')) +
    '<div id="catalog" class="catalog ' + state.layout + '"></div>';
}
function layoutButtons(action, layout) {
  return '<div class="layout-switch">' +
    button(tr('Grille', 'Grid'), action, { value: 'grid', symbol: 'layout-grid', pressed: layout === 'grid', showLabel: true }) +
    button(tr('Liste', 'List'), action, { value: 'list', symbol: 'list', pressed: layout === 'list', showLabel: true }) + '</div>';
}
function renderArtists() {
  const artists = [...new Set(library.map(album => album.artist))].sort((a, b) => a.localeCompare(b))
    .filter(artist => artist.toLocaleLowerCase().includes(state.artistQuery.trim().toLocaleLowerCase()));
  const initials = [...new Set(artists.map(artist => artist[0]))];
  byId('artist-results').innerHTML = '<div class="catalog artists ' + state.artistLayout + '">' +
    (artists.map(artist => {
      const albums = library.filter(album => album.artist === artist);
      return '<button class="album" type="button" data-action="artist" data-value="' + esc(artist) + '">' +
        cover(albums[0]) + '<strong>' + esc(artist) + '</strong><small>' + albumCount(albums) + '</small></button>';
    }).join('') || '<p class="empty">' + tr('Aucun artiste trouvé.', 'No artists found.') + '</p>') +
    '</div><div class="alphabet" aria-label="' + tr('Index alphabétique', 'Alphabetical index') + '">' +
    initials.map(initial => button(initial, 'artist-initial', { value: initial })).join('') + '</div>';
}
function renderTracks(tracks, action) {
  const queue = action === 'queue-track';
  return '<ol class="tracks' + (queue ? ' queue-tracks' : '') + '">' + tracks.map((track, index) => {
    const album = library[track.album];
    const active = queue ? index === state.position : isCurrent(track);
    const pause = active && state.playing;
    return '<li class="track' + (active ? ' active' : '') + '">' +
      button((pause ? tr('Mettre en pause', 'Pause') : tr('Lire', 'Play')) + ' · ' + track.title,
        action, { symbol: pause ? 'pause' : 'play', value: index }) +
      '<small class="time">1.' + (track.index + 1) + '</small>' + (queue ? cover(album) : '') +
      '<div class="track-info"><strong>' + esc(track.title) + '</strong><small>' + esc(album.artist) +
      ' · ' + (queue ? esc(album.title) : audioFormat) + '</small></div><small class="time">' +
      time(track.seconds) + '</small>' +
      (queue ? button(tr('Ajouter à une playlist', 'Add to playlist'), 'queue-add', { symbol: 'list-plus', value: index }) +
        (active ? '<span></span>' : button(tr('Retirer de la file', 'Remove from queue'), 'queue-remove', { symbol: 'x', value: index })) :
        action === 'album-track' ? actionMenu(button(tr('Ajouter à la file', 'Add to queue'), 'enqueue-track', { value: index })) : '') +
      '</li>';
  }).join('') + '</ol>';
}
function actionMenu(items) {
  return '<details class="action-menu"><summary>' + icon('sliders-horizontal') + '<span>' +
    tr('Actions', 'Actions') + '</span>' + icon('chevron-down') +
    '</summary><div class="action-items">' + items + '</div></details>';
}
function renderDetail() {
  const detail = byId('detail');
  detail.hidden = state.view !== 'albums' || state.selected === null;
  if (detail.hidden) {
    detail.innerHTML = '';
    return;
  }
  const album = library[state.selected];
  detail.innerHTML = '<div class="detail-top"><div class="detail-label"><small>' +
    tr('Album sélectionné', 'Selected album') + '</small>' +
    button(tr('Fermer', 'Close'), 'close-album', { symbol: 'x', showLabel: true }) +
    '</div><div class="album-head">' + cover(album) + '<div><h2>' + esc(album.title) +
    '</h2><p class="artist">' + esc(album.artist) + '</p><p class="muted">' + album.year +
    ' · ' + genreName(album.genre) + ' · ' + trackCount(album.tracks) +
    ' · ' + albumDuration(album) + '</p><p class="muted">' + audioFormat +
    '</p></div></div><div class="toolbar">' +
    button(tr('Lire', 'Play'), 'play-album', { symbol: 'play', primary: true, showLabel: true }) +
    actionMenu(button(tr('Ajouter à la file', 'Add to queue'), 'enqueue')) +
    '</div></div><div id="album-tracks">' + renderTracks(album.tracks, 'album-track') + '</div>';
}
const playlistName = list => list.name[Number(state.english)];
function playlistRow(list, index) {
  return '<div class="playlist-row">' +
    (list.tracks.length ? button(tr('Lire la playlist', 'Play playlist'), 'play-playlist', { symbol: 'play', value: index }) :
      '<span class="disabled-icon">' + icon('play') + '</span>') +
    '<span class="playlist-cover">' + icon('disc-3') + '</span><span class="item-copy"><strong>' +
    esc(playlistName(list)) + '</strong><small>' + trackCount(list.tracks) + '</small></span>' +
    button(tr('Renommer', 'Rename'), 'rename-playlist', { symbol: 'pencil', value: index, showLabel: true }) +
    button(tr('Supprimer', 'Delete'), 'delete-playlist', { symbol: 'x', value: index, showLabel: true }) +
    button(tr('Ouvrir', 'Open'), 'playlist', { symbol: 'list', value: index, showLabel: true }) + '</div>';
}
function queueView(title) {
  const track = currentTrack();
  const album = track && library[track.album];
  return heading(title, track ? '<div class="toolbar">' +
    button(tr('Enregistrer en playlist', 'Save as playlist'), 'save-queue', { symbol: 'list-plus', showLabel: true }) +
    button(tr('Vider la file', 'Clear queue'), 'clear-queue', { symbol: 'eraser', showLabel: true }) + '</div>' : '') +
    (track ? '<div class="queue-head">' + cover(album) + '<div class="item-copy"><h2>' + esc(track.title) +
      '</h2><small>' + esc(album.artist) + ' · ' + esc(album.title) + '</small><small>' + audioFormat +
      '</small></div>' + button(tr('Voir l’album', 'View album'), 'show-album', { symbol: 'disc-3', value: album.id, showLabel: true }) +
      '</div>' + renderTracks(state.queue, 'queue-track') :
      '<p class="empty">' + tr('La file de lecture est vide.', 'The play queue is empty.') + '</p>');
}
function renderMain() {
  const main = byId('main');
  const title = viewLabels()[state.view];
  switch (state.view) {
    case 'albums':
      main.innerHTML = albumView();
      renderCatalog();
      break;
    case 'artists': {
      const label = tr('Filtrer les artistes…', 'Filter artists…');
      main.innerHTML = heading(title, '<div class="toolbar"><label class="search">' + icon('search') +
        '<input id="artist-search" type="search" aria-label="' + label + '" placeholder="' + label +
        '" value="' + esc(state.artistQuery) + '"></label>' + layoutButtons('artist-layout', state.artistLayout) +
        '</div>') + '<div id="artist-results"></div>';
      renderArtists();
      break;
    }
    case 'playlists':
      if (state.playlist === null) {
        main.innerHTML = heading(title, '<div class="toolbar">' +
          button(tr('Créer une playlist', 'Create playlist'), 'create-playlist', { symbol: 'plus', primary: true, showLabel: true }) +
          '</div>') + '<div class="playlist-list">' + playlists.map(playlistRow).join('') + '</div>';
      } else {
        const list = playlists[state.playlist];
        main.innerHTML = heading(list.name[Number(state.english)], '<div class="toolbar">' +
          button(tr('Retour aux playlists', 'Back to playlists'), 'navigate', { value: 'playlists', symbol: 'arrow-left' }) +
          (list.tracks.length ? button(tr('Lire la playlist', 'Play playlist'), 'play-playlist',
            { value: state.playlist, symbol: 'play', primary: true, showLabel: true }) : '') +
          '</div>') + '<div id="playlist-tracks">' + renderTracks(list.tracks, 'playlist-track') + '</div>';
      }
      break;
    case 'queue':
      main.innerHTML = queueView(title);
      break;
    case 'settings':
      main.innerHTML = heading(title) + '<div class="section-content"><h2>' +
        tr('Apparence', 'Appearance') + '</h2><div class="variants">' + themeButtons() +
        '</div><h2>' + tr('Langue', 'Language') + '</h2><div>' +
        button(tr('English', 'Français'), 'language') + '</div><p class="muted">' +
        tr('Discothèque fictive · Lecture simulée', 'Fictional library · Simulated playback') + '</p></div>';
      break;
  }
}
function renderPlayer() {
  const track = currentTrack();
  const album = track && library[track.album];
  const playback = state.playing ? tr('Pause', 'Pause') : tr('Lecture', 'Play');
  byId('player').innerHTML = '<div class="timeline"><span id="elapsed" class="time">' +
    time(state.elapsed) + '</span><input id="seek" type="range" min="0" max="' + (track?.seconds ?? 0) +
    '" value="' + state.elapsed + '" style="' + progress(state.elapsed, track?.seconds) + '" aria-label="' + tr('Position de lecture', 'Playback position') +
    '"><span class="time">' + time(track?.seconds ?? 0) + '</span></div><div class="player-controls">' +
    '<div class="now">' + (track ? cover(album) + '<div><strong>' + esc(track.title) +
    '</strong><small>' + esc(album.artist) + ' · ' + esc(album.title) + '</small><small>' + audioFormat +
    '</small></div>' : '<small>' + tr('Aucune piste en lecture', 'No track playing') + '</small>') +
    '</div><div class="transport">' +
    button(tr('Lecture aléatoire', 'Shuffle'), 'shuffle', { symbol: 'shuffle', pressed: state.shuffle }) +
    button(tr('Piste précédente', 'Previous track'), 'previous', { symbol: 'skip-back' }) +
    button(playback, 'toggle', { symbol: state.playing ? 'pause' : 'play', primary: true }) +
    button(tr('Piste suivante', 'Next track'), 'next', { symbol: 'skip-forward' }) +
    button(tr('Répéter la file', 'Repeat queue'), 'repeat', { symbol: 'repeat', pressed: state.repeat }) +
    button(viewLabels().queue, 'navigate', { symbol: 'list-end', value: 'queue' }) +
    '</div><div class="volume">' + button(tr('Activer ou couper le son', 'Toggle mute'), 'mute',
      { symbol: state.volume ? 'volume-2' : 'volume-x' }) +
    '<input id="volume" type="range" min="0" max="100" value="' + state.volume +
    '" style="' + progress(state.volume, 100) + '" aria-label="' + tr('Volume simulé', 'Simulated volume') + '">' +
    button(tr('Plein écran', 'Full screen'), 'fullscreen',
      { symbol: document.fullscreenElement ? 'minimize-2' : 'maximize-2' }) + '</div></div>';
}
function restoreFocus(previous) {
  if (!previous?.dataset.action) return;
  const match = [...document.querySelectorAll('button[data-action]')].find(button =>
    !button.closest('[hidden]') && button.dataset.action === previous.dataset.action &&
    button.dataset.value === previous.dataset.value);
  match?.focus({ preventScroll: true });
}
function render() {
  const previous = document.activeElement;
  document.body.dataset.theme = state.theme;
  document.documentElement.lang = tr('fr', 'en');
  renderNavigation();
  byId('main').classList.toggle('split', state.view === 'albums' && state.selected !== null);
  renderMain();
  renderDetail();
  renderPlayer();
  restoreFocus(previous);
}
function start(tracks, position = 0) {
  if (!tracks.length) return;
  state.queue = [...tracks];
  state.position = position;
  state.elapsed = 0;
  state.playing = true;
}
function playTrack(tracks, index) {
  if (isCurrent(tracks[index])) state.playing = !state.playing;
  else start(tracks, index);
}
function step(direction, automatic = false) {
  const size = state.queue.length;
  if (!size) return;
  if (direction > 0 && state.shuffle && size > 1) {
    state.position = (state.position + 1 + Math.floor(Math.random() * (size - 1))) % size;
  } else {
    const next = state.position + direction;
    if (next >= size && automatic && !state.repeat) {
      state.playing = false;
      return;
    }
    state.position = (next + size) % size;
  }
  state.elapsed = 0;
}
const actions = {
  update() {
    alert(tr('Mise à jour disponible (simulation)', 'Update available (simulation)'));
  },
  navigate(value) {
    state.view = value;
    if (value === 'playlists') state.playlist = null;
  },
  theme(value) {
    if (state.view !== 'settings' || !Object.hasOwn(themes(), value)) return;
    state.theme = value;
    try {
      localStorage.setItem('soliton-webdesign-theme', value);
    } catch {
      // The current session can still use the theme when storage is disabled.
    }
  },
  language() { state.english = !state.english; },
  layout(value) { state.layout = value; },
  'artist-layout'(value) { state.artistLayout = value; },
  'artist-initial'(value) {
    [...document.querySelectorAll('[data-action="artist"]')]
      .find(button => button.dataset.value.startsWith(value))?.scrollIntoView({ block: 'nearest' });
  },
  album(value) { state.selected = Number(value); byId('detail').scrollTop = 0; },
  'close-album'() { state.selected = null; },
  artist(value) {
    Object.assign(state, { view: 'albums', artist: value, query: '', genre: '',
      selected: library.find(album => album.artist === value).id });
  },
  'clear-artist'() { state.artist = ''; },
  playlist(value) { state.playlist = Number(value); },
  'create-playlist'() { createPlaylist([]); },
  'save-queue'() { createPlaylist(state.queue); },
  'rename-playlist'(value) {
    const list = playlists[Number(value)];
    const name = prompt(tr('Nom de la playlist', 'Playlist name'), playlistName(list))?.trim();
    if (name) list.name = [name, name];
  },
  'delete-playlist'(value) {
    if (confirm(tr('Supprimer cette playlist ?', 'Delete this playlist?'))) playlists.splice(Number(value), 1);
  },
  'play-results'() { start(filteredAlbums().flatMap(album => album.tracks)); },
  'play-album'() { start(library[state.selected].tracks); },
  'play-playlist'(value) { start(playlists[Number(value)].tracks); },
  'album-track'(value) { playTrack(library[state.selected].tracks, Number(value)); },
  'playlist-track'(value) { playTrack(playlists[state.playlist].tracks, Number(value)); },
  'queue-track'(value) {
    if (state.position === Number(value)) state.playing = !state.playing;
    else start(state.queue, Number(value));
  },
  enqueue() { state.queue.push(...library[state.selected].tracks); },
  'enqueue-track'(value) { state.queue.push(library[state.selected].tracks[Number(value)]); },
  'show-album'(value) {
    Object.assign(state, { view: 'albums', selected: Number(value), query: '', genre: '', artist: '' });
  },
  'clear-queue'() {
    Object.assign(state, { queue: [], position: 0, elapsed: 0, playing: false });
  },
  'queue-remove'(value) {
    const index = Number(value);
    if (index === state.position) return;
    state.queue.splice(index, 1);
    if (index < state.position) state.position -= 1;
  },
  'queue-add'(value) {
    const track = state.queue[Number(value)];
    const entries = playlists.map((list, index) => (index + 1) + '. ' + playlistName(list));
    const selection = prompt(tr('Numéro de la playlist (0 pour en créer une)', 'Playlist number (0 to create one)') +
      '\n' + entries.join('\n'))?.trim();
    if (!selection) return;
    if (selection === '0') createPlaylist([track]);
    else playlists[Number(selection) - 1]?.tracks.push(track);
  },
  toggle() {
    if (!currentTrack()) return;
    if (state.elapsed >= currentTrack().seconds) state.elapsed = 0;
    state.playing = !state.playing;
  },
  next() { step(1); },
  previous() { step(-1); },
  shuffle() { state.shuffle = !state.shuffle; },
  repeat() { state.repeat = !state.repeat; },
  mute() { state.volume = state.volume ? 0 : 50; },
  fullscreen() {
    const change = document.fullscreenElement ? document.exitFullscreen() : document.documentElement.requestFullscreen?.();
    change?.catch(() => {});
  },
};
function createPlaylist(tracks) {
  const name = prompt(tr('Nom de la playlist', 'Playlist name'), tr('Nouvelle playlist', 'New playlist'))?.trim();
  if (name) playlists.push({ name: [name, name], tracks: [...tracks] });
}
document.addEventListener('click', event => {
  const target = event.target.closest('button[data-action]');
  if (!target) return;
  const previousView = state.view;
  actions[target.dataset.action](target.dataset.value);
  if (target.dataset.action === 'artist-initial') return;
  render();
  if (previousView !== state.view) byId('main').scrollTop = 0;
});
document.addEventListener('input', ({ target }) => {
  switch (target.id) {
    case 'search': state.query = target.value; renderCatalog(); break;
    case 'artist-search': state.artistQuery = target.value; renderArtists(); break;
    case 'genre':
    case 'sort':
      state[target.id] = target.value;
      target.previousElementSibling.textContent = target.selectedOptions[0].textContent;
      renderCatalog();
      break;
    case 'seek':
      state.elapsed = Number(target.value);
      byId('elapsed').textContent = time(state.elapsed);
      target.setAttribute('style', progress(state.elapsed, currentTrack()?.seconds));
      break;
    case 'volume':
      state.volume = Number(target.value);
      target.setAttribute('style', progress(state.volume, 100));
      byId('player').querySelector('[data-action="mute"]').innerHTML = icon(state.volume ? 'volume-2' : 'volume-x');
      break;
  }
});
document.addEventListener('fullscreenchange', renderPlayer);
setInterval(() => {
  if (!state.playing) return;
  state.elapsed += 1;
  if (state.elapsed >= currentTrack().seconds) {
    state.elapsed = currentTrack().seconds;
    step(1, true);
    render();
  } else {
    byId('seek').value = state.elapsed;
    byId('seek').setAttribute('style', progress(state.elapsed, currentTrack().seconds));
    byId('elapsed').textContent = time(state.elapsed);
  }
}, 1000);

render();
