const byId = id => document.getElementById(id);
const state = {
  view: 'albums', theme: 'gpui', english: false, selected: 0,
  query: '', genre: '', sort: 'original', layout: 'grid', artist: '', playlist: null,
  queue: [...library[0].tracks], position: 0, elapsed: 48, playing: false, volume: 65,
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
  queue: tr('File d’attente', 'Queue'), settings: tr('Réglages', 'Settings'),
});
const sectionIcons = { artists: 'users', albums: 'disc-3', playlists: 'list', queue: 'list-end', settings: 'settings' };
const themes = () => ({ gpui: 'GPUI', studio: 'Studio', paper: tr('Papier', 'Paper') });
const genreName = genre => genre === 'Electronic' ? tr('Électronique', 'Electronic') : genre;
const trackCount = tracks => tracks.length + ' ' + tr(tracks.length === 1 ? 'piste' : 'pistes', tracks.length === 1 ? 'track' : 'tracks');
const albumCount = albums => albums.length + (albums.length === 1 ? ' album' : ' albums');
const currentTrack = () => state.queue[state.position];
const isCurrent = track => track.album === currentTrack().album && track.index === currentTrack().index;
const cover = album => '<span class="cover" aria-hidden="true" style="--x:' +
  album.id % 3 * 50 + '%;--y:' + Math.floor(album.id / 3) * 50 + '%"></span>';
function themeButtons() {
  return Object.entries(themes()).map(([value, label]) => button(label, 'theme', { value, pressed: state.theme === value })).join('');
}
function renderNavigation() {
  byId('prototype-label').textContent = tr('Maquette Soliton', 'Soliton prototype');
  byId('variants').setAttribute('aria-label', tr('Design', 'Design'));
  byId('variants').innerHTML = themeButtons();
  byId('language').textContent = tr('EN', 'FR');
  byId('language').setAttribute('aria-label', tr('Afficher en anglais', 'Switch to French'));
  byId('navigation').setAttribute('aria-label', tr('Navigation principale', 'Main navigation'));
  byId('navigation').innerHTML = '<img src="../assets/logos/teal.svg" alt="Soliton">' +
    Object.entries(viewLabels()).map(([value, label]) => button(label, 'navigate', {
      value, symbol: sectionIcons[value], current: state.view === value, showLabel: true,
    })).join('');
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
  return albums;
}
function renderCatalog() {
  byId('catalog').innerHTML = filteredAlbums().map(album =>
    '<button type="button" class="album" data-action="album" data-value="' + album.id +
    '" aria-pressed="' + (state.selected === album.id) + '">' + cover(album) +
    '<strong>' + esc(album.title) + '</strong><small>' + esc(album.artist) + '</small></button>'
  ).join('') || '<p class="empty">' + tr('Aucun album trouvé.', 'No albums found.') + '</p>';
}
function albumView() {
  const search = tr('Rechercher un album, un artiste ou un titre', 'Search for an album, artist or track');
  const genre = tr('Tous les genres', 'All genres');
  return heading('Albums', '<div class="toolbar">' +
    button(tr('Lire les résultats', 'Play results'), 'play-results', { symbol: 'play', primary: true }) +
    '<label class="search">' + icon('search') + '<input id="search" type="search" aria-label="' + search +
    '" placeholder="' + search + '" value="' + esc(state.query) + '"></label>' +
    '<select id="genre" aria-label="' + tr('Genre', 'Genre') + '">' + option('', genre, state.genre) +
    [...new Set(library.map(album => album.genre))].map(value => option(value, genreName(value), state.genre)).join('') + '</select>' +
    '<select id="sort" aria-label="' + tr('Trier les albums', 'Sort albums') + '">' +
    option('original', tr('Collection', 'Collection'), state.sort) +
    option('title', tr('Titre', 'Title'), state.sort) + option('year', tr('Année', 'Year'), state.sort) + '</select>' +
    button(tr('Grille', 'Grid'), 'layout', { value: 'grid', symbol: 'layout-grid', pressed: state.layout === 'grid' }) +
    button(tr('Liste', 'List'), 'layout', { value: 'list', symbol: 'list', pressed: state.layout === 'list' }) +
    '</div>' + (state.artist ? '<div class="toolbar">' +
    button(state.artist + ' ×', 'clear-artist') + '</div>' : '')) +
    '<div id="catalog" class="catalog ' + state.layout + '"></div>';
}
function renderTracks(tracks, action) {
  return '<ol class="tracks">' + tracks.map((track, index) => {
    const active = isCurrent(track);
    const pause = active && state.playing;
    return '<li class="track' + (active ? ' active' : '') + '">' +
      button((pause ? tr('Mettre en pause', 'Pause') : tr('Lire', 'Play')) + ' · ' + track.title,
        action, { symbol: pause ? 'pause' : 'play', value: index }) +
      '<small class="time">' + (index + 1) + '</small><div class="track-info"><strong>' +
      esc(track.title) + '</strong><small>FLAC · 96 kHz · 24 bit · ' +
      esc(library[track.album].artist) + '</small></div><small class="time">' + time(track.seconds) + '</small></li>';
  }).join('') + '</ol>';
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
    button(tr('Fermer l’album', 'Close album'), 'close-album', { symbol: 'x' }) +
    '</div><div class="album-head">' + cover(album) + '<div><h2>' + esc(album.title) +
    '</h2><p class="artist">' + esc(album.artist) + '</p><p class="muted">' + album.year +
    ' · ' + genreName(album.genre) + ' · ' + trackCount(album.tracks) +
    ' · ' + time(album.tracks.reduce((sum, track) => sum + track.seconds, 0)) +
    '</p><p class="muted">FLAC · 96 kHz · 24 bit</p></div></div><div class="toolbar">' +
    button(tr('Lire l’album', 'Play album'), 'play-album', { symbol: 'play', primary: true }) +
    button(tr('Ajouter à la file', 'Add to queue'), 'enqueue', { symbol: 'list-plus' }) +
    '</div></div><div id="album-tracks">' + renderTracks(album.tracks, 'album-track') + '</div>';
}
function collection(label, summary, album, action, value) {
  return '<button type="button" class="collection" data-action="' + action + '" data-value="' +
    esc(value) + '">' + cover(album) + '<strong>' + esc(label) + '</strong><small>' +
    esc(summary) + '</small></button>';
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
      const artists = [...new Set(library.map(album => album.artist))].sort((a, b) => a.localeCompare(b));
      main.innerHTML = heading(title) + '<div class="section-content">' +
        artists.map(artist => {
          const albums = library.filter(album => album.artist === artist);
          return collection(artist, albumCount(albums), albums[0], 'artist', artist);
        }).join('') + '</div>';
      break;
    }
    case 'playlists':
      if (state.playlist === null) {
        main.innerHTML = heading(title) + '<div class="section-content">' + playlists.map((list, index) =>
          collection(list.name[Number(state.english)], trackCount(list.tracks), library[list.tracks[0].album], 'playlist', index)
        ).join('') + '</div>';
      } else {
        const list = playlists[state.playlist];
        main.innerHTML = heading(list.name[Number(state.english)], '<div class="toolbar">' +
          button(tr('Retour aux playlists', 'Back to playlists'), 'navigate', { value: 'playlists', symbol: 'arrow-left' }) +
          button(tr('Lire la playlist', 'Play playlist'), 'play-playlist', { symbol: 'play', primary: true }) +
          '</div>') + '<div id="playlist-tracks">' + renderTracks(list.tracks, 'playlist-track') + '</div>';
      }
      break;
    case 'queue':
      main.innerHTML = heading(title, '<p class="muted toolbar">' + trackCount(state.queue) +
        '</p>') + '<div id="queue-tracks">' + renderTracks(state.queue, 'queue-track') + '</div>';
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
  const album = library[track.album];
  const playback = state.playing ? tr('Pause', 'Pause') : tr('Lecture', 'Play');
  byId('player').innerHTML = '<div class="timeline"><span id="elapsed" class="time">' +
    time(state.elapsed) + '</span><input id="seek" type="range" min="0" max="' + track.seconds +
    '" value="' + state.elapsed + '" aria-label="' + tr('Position de lecture', 'Playback position') +
    '"><span class="time">' + time(track.seconds) + '</span></div><div class="player-controls">' +
    '<div class="now">' + cover(album) + '<div><strong>' + esc(track.title) +
    '</strong><small>' + esc(album.artist) + ' · ' + esc(album.title) + '</small></div></div><div class="transport">' +
    button(tr('Lecture aléatoire', 'Shuffle'), 'shuffle', { symbol: 'shuffle', pressed: state.shuffle }) +
    button(tr('Piste précédente', 'Previous track'), 'previous', { symbol: 'skip-back' }) +
    button(playback, 'toggle', { symbol: state.playing ? 'pause' : 'play', primary: true }) +
    button(tr('Piste suivante', 'Next track'), 'next', { symbol: 'skip-forward' }) +
    button(tr('Répéter la file', 'Repeat queue'), 'repeat', { symbol: 'repeat', pressed: state.repeat }) +
    button(viewLabels().queue, 'navigate', { symbol: 'list-end', value: 'queue' }) +
    '</div><div class="volume">' + button(tr('Activer ou couper le son', 'Toggle mute'), 'mute',
      { symbol: state.volume ? 'volume-2' : 'volume-x' }) +
    '<input id="volume" type="range" min="0" max="100" value="' + state.volume +
    '" aria-label="' + tr('Volume simulé', 'Simulated volume') + '"></div></div>';
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
  navigate(value) {
    state.view = value;
    if (value === 'playlists') state.playlist = null;
  },
  theme(value) { state.theme = value; },
  language() { state.english = !state.english; },
  layout(value) { state.layout = value; },
  album(value) { state.selected = Number(value); byId('detail').scrollTop = 0; },
  'close-album'() { state.selected = null; },
  artist(value) {
    Object.assign(state, { view: 'albums', artist: value, query: '', genre: '',
      selected: library.find(album => album.artist === value).id });
  },
  'clear-artist'() { state.artist = ''; },
  playlist(value) { state.playlist = Number(value); },
  'play-results'() { start(filteredAlbums().flatMap(album => album.tracks)); },
  'play-album'() { start(library[state.selected].tracks); },
  'play-playlist'() { start(playlists[state.playlist].tracks); },
  'album-track'(value) { playTrack(library[state.selected].tracks, Number(value)); },
  'playlist-track'(value) { playTrack(playlists[state.playlist].tracks, Number(value)); },
  'queue-track'(value) {
    if (state.position === Number(value)) state.playing = !state.playing;
    else start(state.queue, Number(value));
  },
  enqueue() { state.queue.push(...library[state.selected].tracks); },
  toggle() {
    if (state.elapsed >= currentTrack().seconds) state.elapsed = 0;
    state.playing = !state.playing;
  },
  next() { step(1); },
  previous() { step(-1); },
  shuffle() { state.shuffle = !state.shuffle; },
  repeat() { state.repeat = !state.repeat; },
  mute() { state.volume = state.volume ? 0 : 65; },
};
document.addEventListener('click', event => {
  const target = event.target.closest('button[data-action]');
  if (!target) return;
  const previousView = state.view;
  actions[target.dataset.action](target.dataset.value);
  render();
  if (previousView !== state.view) byId('main').scrollTop = 0;
});
document.addEventListener('input', ({ target }) => {
  switch (target.id) {
    case 'search': state.query = target.value; renderCatalog(); break;
    case 'genre':
    case 'sort': state[target.id] = target.value; renderCatalog(); break;
    case 'seek':
      state.elapsed = Number(target.value);
      byId('elapsed').textContent = time(state.elapsed);
      break;
    case 'volume':
      state.volume = Number(target.value);
      byId('player').querySelector('[data-action="mute"]').innerHTML = icon(state.volume ? 'volume-2' : 'volume-x');
      break;
  }
});
setInterval(() => {
  if (!state.playing) return;
  state.elapsed += 1;
  if (state.elapsed >= currentTrack().seconds) {
    state.elapsed = currentTrack().seconds;
    step(1, true);
    render();
  } else {
    byId('seek').value = state.elapsed;
    byId('elapsed').textContent = time(state.elapsed);
  }
}, 1000);

render();
