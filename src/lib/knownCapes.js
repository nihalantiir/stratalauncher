// A catalog of Minecraft Java Edition profile capes for the "Try on"
// browser, so someone can preview a cape they don't own without Strata
// ever claiming to equip it.
//
// Every URL is a real `textures.minecraft.net/texture/<hash>` link,
// individually confirmed live before being hardcoded here.
//
// Scoped to real Java Edition profile capes only, not Bedrock Marketplace
// skin-pack cosmetics or datamined/unused textures.
export const KNOWN_CAPES = [
  // Ownership capes: obtainable broadly/ongoing, not a one-time grant.
  { id: 'vanilla', name: 'Vanilla', category: 'ownership', url: 'http://textures.minecraft.net/texture/f9a76537647989f9a0b6d001e320dac591c359e9e61a31f4ce11c88f207f0ad4' },
  { id: 'pan', name: 'Pan', category: 'ownership', url: 'http://textures.minecraft.net/texture/28de4a81688ad18b49e735a273e086c18f1e3966956123ccb574034c06f5d336' },
  { id: 'common', name: 'Common', category: 'ownership', url: 'http://textures.minecraft.net/texture/5ec930cdd2629c8771655c60eebeb867b4b6559b0e6d3bc71c40c96347fa03f0' },
  { id: 'migrator', name: 'Migrator', category: 'ownership', url: 'http://textures.minecraft.net/texture/2340c0e03dd24a11b15a8b33c2a7e9e32abb2051b2481d0ba7defd635ca7a933' },

  // Virtual Event: an online-only campaign, livestream, or web giveaway.
  { id: 'cherry_blossom', name: 'Cherry Blossom', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/afd553b39358a24edfe3b8a9a939fa5fa4faa4d9a9c3d6af8eafb377fa05c2bb' },
  { id: 'anniversary15', name: '15th Anniversary', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/cd9d82ab17fd92022dbd4a86cde4c382a7540e117fae7b9a2853658505a80625' },
  { id: 'home', name: 'Home', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/1de21419009db483900da6298a1e6cbf9f1bc1523a0dcdc16263fab150693edd' },
  { id: 'menace', name: 'Menace', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/dbc21e222528e30dc88445314f7be6ff12d3aeebc3c192054fba7e3b3f8c77b1' },
  { id: 'zombie_horse', name: 'Zombie Horse', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/a3f6e4f14801f3ea55e3d95b9b4ef3b5e8802d947f669de93d6ec4b9354a436b' },
  { id: 'purple_heart', name: 'Purple Heart', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/cb40a92e32b57fd732a00fc325e7afb00a7ca74936ad50d8e860152e482cfbde' },
  { id: 'founders', name: "Founder's", category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/99aba02ef05ec6aa4d42db8ee43796d6cd50e4b2954ab29f0caeb85f96bf52a1' },
  { id: 'copper', name: 'Copper', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/5e6f3193e74cd16cdd6637d9bae5484e3a37ff2a14c2d157c659a07810b1bdca' },
  { id: 'builder', name: 'Builder', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/2c579968c64c1719740fd8c2a451461879b238002574fce48f7d1a7c36a1c7d4' },
  { id: 'hero', name: 'Hero', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/5fc841ae5a06cb385851e63ee13dccbeb913e8cbffe6a9a1168b69ff601d2188' },
  { id: 'twisted', name: 'Twisted', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/24aafc451aa2cc34ddc7265211678585c0ef4da4d32edb75ecec1bd8b5408381' },
  { id: 'mcc15', name: 'MCC 15th Year', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/56c35628fe1c4d59dd52561a3d03bfa4e1a76d397c8b9c476c2f77cb6aebb1df' },
  { id: 'followers', name: "Follower's", category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/569b7f2a1d00d26f30efe3f9ab9ac817b1e6d35f4f3cfb0324ef2d328223d350' },
  { id: 'mojang_office', name: 'Mojang Office', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/5c29410057e32abec02d870ecb52ec25fb45ea81e785a7854ae8429d7236ca26' },
  { id: 'yearn', name: 'Yearn', category: 'virtualEvent', url: 'http://textures.minecraft.net/texture/308b32a9e303155a0b4262f9e5483ad4a22e3412e84fe8385a0bdd73dc41fa89' },

  // Event: a real-world/in-person grant, MineCon, a studio visit, an exhibit.
  { id: 'minecon2011', name: 'MineCon 2011', category: 'event', url: 'http://textures.minecraft.net/texture/953cac8b779fe41383e675ee2b86071a71658f2180f56fbce8aa315ea70e2ed6' },
  { id: 'minecon2012', name: 'MineCon 2012', category: 'event', url: 'http://textures.minecraft.net/texture/a2e8d97ec79100e90a75d369d1b3ba81273c4f82bc1b737e934eed4a854be1b6' },
  { id: 'minecon2013', name: 'MineCon 2013', category: 'event', url: 'http://textures.minecraft.net/texture/153b1a0dfcbae953cdeb6f2c2bf6bf79943239b1372780da44bcbb29273131da' },
  { id: 'minecon2015', name: 'MineCon 2015', category: 'event', url: 'http://textures.minecraft.net/texture/b0cc08840700447322d953a02b965f1d65a13a603bf64b17c803c21446fe1635' },
  { id: 'minecon2016', name: 'MineCon 2016', category: 'event', url: 'http://textures.minecraft.net/texture/e7dfea16dc83c97df01a12fabbd1216359c0cd0ea42f9999b6e97c584963e980' },
  { id: 'crafter', name: 'Crafter', category: 'event', url: 'http://textures.minecraft.net/texture/479eacefa3cdd7aca94207f36c0dd449653ddf259daf40544a5866baf05eee22' },
  { id: 'moonlight_trail', name: 'Moonlight Trail', category: 'event', url: 'http://textures.minecraft.net/texture/fe8a02dfe9e390e44ff33d69feef9d3943f76d3901015bbd50f0b67722d288bd' },
  { id: 'experience', name: 'Minecraft Experience', category: 'event', url: 'http://textures.minecraft.net/texture/7658c5025c77cfac7574aab3af94a46a8886e3b7722a895255fbf22ab8652434' },

  // Volunteer/competition: earned through community contribution, not purchase.
  { id: 'translator', name: 'Translator', category: 'volunteer', url: 'http://textures.minecraft.net/texture/1bf91499701404e21bd46b0191d63239a4ef76ebde88d27e4d430ac211df681e' },
  { id: 'translator_cn', name: 'Chinese Translator', category: 'volunteer', url: 'http://textures.minecraft.net/texture/2262fb1d24912209490586ecae98aca8500df3eff91f2a07da37ee524e7e3cb6' },
  { id: 'moderator', name: 'Moderator', category: 'volunteer', url: 'http://textures.minecraft.net/texture/ae677f7d98ac70a533713518416df4452fe5700365c09cf45d0d156ea9396551' },
  { id: 'realms_mapmaker', name: 'Realms MapMaker', category: 'volunteer', url: 'http://textures.minecraft.net/texture/17912790ff164b93196f08ba71d0e62129304776d0f347334f8a6eae509f8a56' },
  { id: 'scrolls_champion', name: 'Scrolls Champion', category: 'volunteer', url: 'http://textures.minecraft.net/texture/3efadf6510961830f9fcc077f19b4daf286d502b5f5aafbd807c7bbffcaca245' },
  { id: 'cobalt', name: 'Cobalt', category: 'volunteer', url: 'http://textures.minecraft.net/texture/ca35c56efe71ed290385f4ab5346a1826b546a54d519e6a3ff01efa01acce81' },

  // Staff: Mojang/Microsoft employee capes, never obtainable by players.
  { id: 'mojang', name: 'Mojang', category: 'staff', url: 'http://textures.minecraft.net/texture/5786fe99be377dfb6858859f926c4dbc995751e91cee373468c5fbf4865e7151' },
  { id: 'classic_mojang', name: 'Classic Mojang', category: 'staff', url: 'http://textures.minecraft.net/texture/8f120319222a9f4a104e2f5cb97b2cda93199a2ee9e1585cb8d09d6f687cb761' },
  { id: 'mojang_studios', name: 'Mojang Studios', category: 'staff', url: 'http://textures.minecraft.net/texture/9e507afc56359978a3eb3e32367042b853cddd0995d17d0da995662913fb00f7' },

  // Personal grants: given to one specific person, never a general program.
  { id: 'turtle', name: 'Turtle', category: 'personal', url: 'http://textures.minecraft.net/texture/5048ea61566353397247d2b7d946034de926b997d5e66c86483dfb1e031aee95' },
  { id: 'bacon', name: 'Bacon', category: 'personal', url: 'http://textures.minecraft.net/texture/fd14214cd8073059e93d9c626260f5df85e5a959181537119df56cadaf5002cc' },
  { id: 'birthday', name: 'Birthday', category: 'personal', url: 'http://textures.minecraft.net/texture/2056f2eebd759cce93460907186ef44e9192954ae12b227d817eb4b55627a7fc' },
  { id: 'millionth_customer', name: 'Millionth Customer', category: 'personal', url: 'http://textures.minecraft.net/texture/70efffaf86fe5bc089608d3cb297d3e276b9eb7a8f9f2fe6659c23a2d8b18edf' },
  { id: 'db', name: 'dB', category: 'personal', url: 'http://textures.minecraft.net/texture/bcfbe84c6542a4a5c213c1cacf8979b5e913dcb4ad783a8b80e3c4a7d5c8bdac' },
  { id: 'snowman', name: 'Snowman', category: 'personal', url: 'http://textures.minecraft.net/texture/23ec737f18bfe4b547c95935fc297dd767bb84ee55bfd855144d279ac9bfd9fe' },
  { id: 'cheapsh0t', name: "Cheapsh0t's", category: 'personal', url: 'http://textures.minecraft.net/texture/ca29f5dd9e94fb1748203b92e36b66fda80750c87ebc18d6eafdb0e28cc1d05f' },
  { id: 'spade', name: 'Spade', category: 'personal', url: 'http://textures.minecraft.net/texture/2e002d5e1758e79ba51d08d92a0f3a95119f2f435ae7704916507b6c565a7da8' },
  { id: 'prismarine', name: 'Prismarine', category: 'personal', url: 'http://textures.minecraft.net/texture/d8f8d13a1adf9636a16c31d47f3ecc9bb8d8533108aa5ad2a01b13b1a0c55eac' },
  { id: 'valentine', name: 'Valentine', category: 'personal', url: 'http://textures.minecraft.net/texture/e578ef995fabcf0a94768f9651ac3aaba30c59ef85d2438e9b3e0cc1d810652b' },
  { id: 'oxeye', name: 'Oxeye', category: 'personal', url: 'http://textures.minecraft.net/texture/7706b5f5fc90329691e59277dcc66ba20572219fa8e5da472afd5235fad12cc8' },
  { id: 'blueprint', name: 'Blueprint', category: 'personal', url: 'http://textures.minecraft.net/texture/fdcf48f01ec480d1d7cbec27f7ddce48c9da2be6724641109444dae58d4cd013' },
];

export const CAPE_CATEGORIES = [
  { id: 'ownership', label: 'Ownership' },
  { id: 'virtualEvent', label: 'Virtual Event' },
  { id: 'event', label: 'Event' },
  { id: 'volunteer', label: 'Volunteer & competition' },
  { id: 'staff', label: 'Staff' },
  { id: 'personal', label: 'Personal grants' },
];
