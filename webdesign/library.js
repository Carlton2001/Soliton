const library = [
  ['Marées nocturnes', 'Ligne d’Horizon', 2024, 'Ambient', ['Mare Incognita', 'Low Tide', 'After the Beacon', 'Pressure Lines', 'A Distant Shore', 'Last Light']],
  ['Heure bleue', 'Mina Sorel', 2021, 'Dream pop', ['Blue Hour', 'Silver Glass', 'Out of Focus', 'Quiet Rooms', 'Before Dawn']],
  ['Sous la mousse', 'Les Formes Lentes', 2025, 'Ambient', ['Understory', 'Petrichor', 'Green Canopy', 'Ferns', 'Soft Ground']],
  ['Sable magnétique', 'Atlas Noir', 2023, 'Electronic', ['Red Dunes', 'Field Notes', 'Magnetic Sand', 'Antenna', 'Dry Lightning']],
  ['Latitude zéro', 'Ligne d’Horizon', 2019, 'Ambient', ['White Latitude', 'Drift Ice', 'The Crossing', 'Cold Front', 'Northbound']],
  ['Corps célestes', 'Vesper Module', 2025, 'Electronic', ['Low Orbit', 'Stationkeeping', 'Umbra', 'Solar Wind', 'Return Signal']],
  ['Néon liquide', 'Vesper Module', 2022, 'Electronic', ['Wet Asphalt', 'City Loop', 'Liquid Neon', 'Electric Rain', 'Night Bus']],
  ['Les oiseaux de papier', 'Mina Sorel', 2018, 'Dream pop', ['Paper Wings', 'Folding Light', 'Small Hours', 'Windowsill', 'Southbound']],
  ['Chlorophylle', 'Les Formes Lentes', 2020, 'Jazz', ['Spore Print', 'Black Soil', 'Veins', 'Gold Dust', 'Gentle Growth']],
].map(([title, artist, year, genre, tracks], id) => ({
  id, title, artist, year, genre,
  tracks: tracks.map((title, index) => ({
    album: id, index, title, seconds: 164 + (id * 53 + index * 37) % 181,
  })),
}));

const playlists = [
  { name: ['Après minuit', 'After Midnight'], tracks: [library[0].tracks[0], library[1].tracks[2], library[5].tracks[1], library[6].tracks[4]] },
  { name: ['Au vert', 'Into the Green'], tracks: [library[2].tracks[0], library[8].tracks[1], library[4].tracks[0]] },
];
