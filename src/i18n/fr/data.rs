//! French texts of the data tables.

use crate::i18n::data::{ChallengeText, DataTexts, EnemyText, Named, UnitText};

#[rustfmt::skip]
const UNITS: &[(&str, UnitText)] = &[
    ("soldier", UnitText { name: "Soldat", unlock_name: "Conscription", mult_name: "Grenades à fragmentation" }),
    ("mortar", UnitText { name: "Mortier", unlock_name: "Mortiers", mult_name: "Obus explosifs" }),
    ("artillery", UnitText { name: "Artilleur", unlock_name: "Artillerie lourde", mult_name: "Obus perforants" }),
    ("tank", UnitText { name: "Tank", unlock_name: "Divisions blindées", mult_name: "Canons de 120" }),
    ("bomber", UnitText { name: "Bombardier", unlock_name: "Escadrilles", mult_name: "Bombes au napalm" }),
    ("sub", UnitText { name: "Sous-marin nucléaire", unlock_name: "Flotte sous-marine", mult_name: "Torpilles atomiques" }),
    ("icbm", UnitText { name: "Silo ICBM", unlock_name: "Silos ICBM", mult_name: "Têtes multiples" }),
    ("reactor", UnitText { name: "Réacteur à fusion", unlock_name: "Réacteurs à fusion", mult_name: "Plasma confiné" }),
    ("blackhole", UnitText { name: "Trou noir de poche", unlock_name: "Singularité", mult_name: "Horizon des événements" }),
    ("moussaka", UnitText { name: "Moussaka apprivoisée", unlock_name: "Recette de mamie", mult_name: "Béchamel enrichie" }),
    ("savior", UnitText { name: "Sauveur du monde", unlock_name: "Ceinture noire", mult_name: "Poings de fer" }),
    ("diamond", UnitText { name: "Diamant blanc", unlock_name: "Mine de diamants", mult_name: "Taille en brillant" }),
    ("croc", UnitText { name: "Croco en caoutchouc", unlock_name: "Rizière hantée", mult_name: "Mâchoires en mousse" }),
    ("mummy", UnitText { name: "Momie de la falaise", unlock_name: "Couvercle sans sarcophage", mult_name: "Bandelettes au faux sang" }),
    ("ninja", UnitText { name: "Ninja d'Interpol", unlock_name: "Opération Pastèque", mult_name: "Baguettes de combat" }),
];

#[rustfmt::skip]
const PTALENTS: &[(&str, Named)] = &[
    ("p_cap", Named { name: "Expansion du cœur",
              desc: "Niveau max ×{} ({} → {})." }),
    ("p_prod", Named { name: "Uranium appauvri",
              desc: "Production ×{} par rang (multiplicatif)." }),
    ("p_click", Named { name: "Doigt nucléaire",
              desc: "Puissance de détonation ×{} par rang." }),
    ("p_pt", Named { name: "Mémoire génétique",
              desc: "Commence chaque run avec +{} PT par rang." }),
    ("p_vets", Named { name: "Vétérans",
              desc: "Commence avec la Conscription et {} soldats par rang." }),
    ("p_keep", Named { name: "Automates persistants",
              desc: "Auto-détonateur, Quartier-maître et Marché noir sont conservés au reset." }),
    ("p_iso", Named { name: "Centrifugeuses",
              desc: "+{} % d'isotopes gagnés par rang." }),
    ("p_drop", Named { name: "Contrebandiers",
              desc: "Ravitaillements +{} % plus efficaces par rang." }),
    ("p_offline", Named { name: "Sommeil radioactif",
              desc: "Progression hors-ligne : {} % de la production par rang ({} h max)." }),
    ("p_dna", Named { name: "ADN instable",
              desc: "+{} mutation proposée au choix par rang." }),
];

#[rustfmt::skip]
const STALENTS: &[(&str, Named)] = &[
    ("s_prod", Named { name: "Fusion stellaire",
              desc: "Production ×{} par rang." }),
    ("s_iso", Named { name: "Nucléosynthèse",
              desc: "Isotopes gagnés ×{} par rang." }),
    ("s_core", Named { name: "Cœur dense",
              desc: "Expansion du cœur coûte {} % de moins par rang." }),
    ("s_crit", Named { name: "Pulsar",
              desc: "La masse critique se charge {}× plus vite par rang." }),
    ("s_auto", Named { name: "Chaîne de montage",
              desc: "Achète automatiquement les talents de run." }),
    ("s_keep", Named { name: "Cœur stellaire",
              desc: "Garde {} rang d'Expansion du cœur après une supernova, par rang." }),
];

#[rustfmt::skip]
const CHALLENGES: &[(&str, ChallengeText)] = &[
    ("pacifist", ChallengeText {
        name: "Pacifiste",
        desc: "Aucune unité autorisée. Atteins le niveau {0}.",
        reward: "Détonation ×{} par rang + skin Vert radioactif",
    }),
    ("onearm", ChallengeText {
        name: "Manchot",
        desc: "Ni ESPACE ni auto-détonateur ({1} soldats offerts). Atteins le niveau {0}.",
        reward: "Production ×{} par rang + skin Arc-en-ciel",
    }),
    ("race", ChallengeText {
        name: "Course",
        desc: "Atteins le niveau {0} en moins de {1} minutes.",
        reward: "Isotopes ×{} par rang + skin Champignon de Mario",
    }),
    ("inflation", ChallengeText {
        name: "Inflation",
        desc: "Les unités coûtent {1} fois plus cher. Atteins le niveau {0}.",
        reward: "Coût des unités ×{} par rang + skin Or",
    }),
    ("blind", ChallengeText {
        name: "Brouillard de guerre",
        desc: "Niveau, radiation et production sont masqués. Atteins le niveau {0}.",
        reward: "+{} mutation proposée par rang + skin Fantôme",
    }),
    ("watched", ChallengeText {
        name: "Sous surveillance",
        desc: "Un inspecteur toutes les {1} à {2} s, gardes et brouillard inefficaces. Atteins le niveau {0}.",
        reward: "Butin des inspecteurs chassés ×{} par rang + skin Néon",
    }),
    ("ascete", ChallengeText {
        name: "Ascète",
        desc: "Seuls les talents qui débloquent des unités sont autorisés. Atteins le niveau {0}.",
        reward: "+{} PT au début de chaque run par rang + skin Glace",
    }),
    ("hothead", ChallengeText {
        name: "Tête brûlée",
        desc: "Surchauffe dès {1} appuis/s, pendant {2} s, même avec les Doigts d'amiante. Atteins le niveau {0}.",
        reward: "Seuil de surchauffe +{} appuis/s par rang + skin Lave",
    }),
    ("invasion", ChallengeText {
        name: "Invasion",
        desc: "Un ennemi toutes les {1} à {2} s, tous types dès le début. Atteins le niveau {0}.",
        reward: "Marche des ennemis ×{} par rang + skin Camouflage",
    }),
    ("sterile", ChallengeText {
        name: "Stérile",
        desc: "Aucune mutation ne t'est proposée. Atteins le niveau {0}.",
        reward: "Masse critique ×{} plus rapide par rang + skin Cristal",
    }),
    ("unlucky", ChallengeText {
        name: "Chat noir",
        desc: "Ni critiques, ni masse critique, ni ravitaillements. Atteins le niveau {0}.",
        reward: "Chance de critique +{} % par rang + skin Obsidienne",
    }),
    ("nanar", ChallengeText {
        name: "Nanarland",
        desc: "Effets spéciaux en carton : 1 détonation sur {1} fait pschitt, budget de série Z (production ÷{2}). \
                   Atteins le niveau {0}.",
        reward: "Critiques +{} par rang (×{} sans) + skin Carton-pâte",
    }),
    ("mono", ChallengeText {
        name: "Monoculture",
        desc: "Une seule sorte d'unité, la première que tu achètes (sans Vétérans). Atteins le niveau {0}.",
        reward: "Paliers d'unités +{} par rang (×{} sans) + skin Mousse",
    }),
    ("halflife", ChallengeText {
        name: "Demi-vie",
        desc: "Radiation stockée et progression du niveau fondent de moitié toutes les {1} s, l'armée perd {2} % \
                   de ses unités par seconde. Atteins le niveau {0}.",
        reward: "Combo max +{} par rang + skin Uranium",
    }),
    ("strike", ChallengeText {
        name: "Grève générale",
        desc: "L'armée ne produit que {1} s après chaque appui sur ESPACE (l'auto-détonateur ne compte pas). \
                   Atteins le niveau {0}.",
        reward: "Onde de choc ×{} par rang + skin Crépuscule",
    }),
    ("moussaka", ChallengeText {
        name: "L'attaque de la moussaka géante",
        desc: "Une moussaka géante dévore la ville : production divisée par sa taille, qui grossit de 1 \
                   toutes les {1} s. Atteins le niveau {0}.",
        reward: "Débloque la Moussaka apprivoisée, sa production ×{} par rang + skin Moussaka géante",
    }),
    ("savior", ChallengeText {
        name: "L'homme qui sauve le monde",
        desc: "Le héros fait tout lui-même : l'armée ne produit rien seule, chaque appui sur ESPACE \
                   (auto-détonateur compris) lui fait cracher {1} s de production. Atteins le niveau {0}.",
        reward: "Débloque le Sauveur du monde, sa production ×{} par rang + skin Sauveur du monde",
    }),
    ("whitefire", ChallengeText {
        name: "White Fire",
        desc: "Baston homérique : chaque appui sur ESPACE a {1} % de risque de déclencher une bagarre qui \
                   coûte {2} % de ta radiation. Atteins le niveau {0}.",
        reward: "Débloque le Diamant blanc, sa production ×{} par rang + skin Feu blanc",
    }),
    ("crocodile", ChallengeText {
        name: "Crocodile Fury",
        desc: "Toutes les {1} s, le crocodile en caoutchouc dévore {2} % d'une sorte d'unité au hasard \
                   (une unité seule est épargnée). \
                   Atteins le niveau {0}.",
        reward: "Débloque le Croco en caoutchouc, sa production ×{} par rang + skin Rizière",
    }),
    ("devil", ChallengeText {
        name: "Devil Story",
        desc: "Le stock-shot du cheval noir repasse en boucle : une seule unité à la fois, toutes les {1} s \
                   au plus. Atteins le niveau {0}.",
        reward: "Débloque la Momie de la falaise, sa production ×{} par rang + skin Gore-mandie",
    }),
    ("ninja", ChallengeText {
        name: "Black Ninja",
        desc: "Ninja: Silent Assassin : pas de combo, chaque appui frappe seul. Atteins le niveau {0}.",
        reward: "Débloque le Ninja d'Interpol, sa production ×{} par rang + skin Black Carambar",
    }),
];

const SKINS: &[(&str, &str)] = &[
    ("classic", "Classique (feu)"),
    ("toxic", "Vert radioactif"),
    ("rainbow", "Arc-en-ciel"),
    ("mario", "Champignon de Mario"),
    ("plasma", "Plasma (5 prestiges)"),
    ("gold", "Or massif"),
    ("ghost", "Fantôme"),
    ("neon", "Néon"),
    ("ice", "Glace"),
    ("lava", "Lave"),
    ("camo", "Camouflage"),
    ("crystal", "Cristal"),
    ("obsidian", "Obsidienne"),
    ("carton", "Carton-pâte"),
    ("moss", "Mousse"),
    ("uranium", "Uranium"),
    ("sunset", "Crépuscule"),
    ("moussaka", "Moussaka géante"),
    ("savior", "Sauveur du monde"),
    ("whitefire", "Feu blanc"),
    ("ricefield", "Rizière"),
    ("goremandie", "Gore-mandie"),
    ("carambar", "Black Carambar"),
];
/// Run talents (the unit talents are generated: unit_unlock, unit_mult).
#[rustfmt::skip]
const TALENTS: &[(&str, Named)] = &[
    ("t_click", Named { name: "Détonateur renforcé",
              desc: "+{} de puissance de détonation de base par rang." }),
    ("t_crit", Named { name: "Fission critique",
              desc: "+{} % de chance de détonation critique par rang." }),
    ("t_combo", Named { name: "Réaction en chaîne",
              desc: "Combo max +{} par rang (×{} → ×{})." }),
    ("t_cool", Named { name: "Refroidissement",
              desc: "Seuil de surchauffe +{} appuis/s par rang." }),
    ("t_auto", Named { name: "Auto-détonateur",
              desc: "Appuie tout seul sur ESPACE : {} fois/s par rang." }),
    ("t_shock", Named { name: "Onde de choc",
              desc: "Chaque appui ajoute {} % de la production/s par rang." }),
    ("l_global", Named { name: "Propagande",
              desc: "Production globale +{} % par rang." }),
    ("l_synergy", Named { name: "Esprit de corps",
              desc: "+{} % de production par unité possédée, par rang." }),
    ("l_guard", Named { name: "Gardes",
              desc: "Des gardes chassent automatiquement les inspecteurs de l'ONU." }),
    ("l_discount", Named { name: "Marché noir",
              desc: "Coût des unités -{} % par rang." }),
    ("l_autobuy", Named { name: "Quartier-maître",
              desc: "Achète automatiquement les unités ({} types de plus par rang)." }),
    ("l_radar", Named { name: "Radar",
              desc: "Ravitaillements +{} % plus fréquents et avions plus lents." }),
    ("l_enrich", Named { name: "Enrichissement",
              desc: "La masse critique se charge {} % plus vite par rang." }),
];

const CHESTS: &[(&str, &str)] = &[
    ("frenzy", "Frénésie"),
    ("clickfrenzy", "Détonateur fou"),
    ("lump", "Caisse d'uranium"),
    ("pt", "Manuel tactique"),
    ("gauge", "Plutonium"),
    ("army", "Renforts"),
    ("shield", "Pots-de-vin"),
];

const BRANCHES: &[(&str, &str)] = &[
    ("det", "DÉTONATION"),
    ("inf", "INFANTERIE"),
    ("log", "LOGISTIQUE"),
];

#[rustfmt::skip]
const MUTATIONS: &[(&str, Named)] = &[
    ("spores", Named { name: "Spores radioactives",
              desc: "+{} % de production par soldat." }),
    ("twin", Named { name: "Champignon jumeau",
              desc: "Détonation ×{} (et un deuxième chapeau !)." }),
    ("acid", Named { name: "Pluie acide",
              desc: "Unités -{} % de coût, mais détonation ÷{}." }),
    ("unstable", Named { name: "Cœur instable",
              desc: "La masse critique se charge {}× plus vite." }),
    ("fallout", Named { name: "Retombées massives",
              desc: "Production ×{}." }),
    ("magnet", Named { name: "Aimant à colis",
              desc: "Ravitaillements {}× plus fréquents." }),
    ("berserk", Named { name: "Rage atomique",
              desc: "Combo max +{}." }),
    ("fog", Named { name: "Brouillard radioactif",
              desc: "Les inspecteurs ne trouvent plus le champignon." }),
    ("mitosis", Named { name: "Mitose",
              desc: "+{} PT supplémentaire par niveau." }),
    ("asbestos", Named { name: "Doigts d'amiante",
              desc: "Plus jamais de surchauffe." }),
    ("glow", Named { name: "Lueur verte",
              desc: "+{} % de chance de détonation critique." }),
    ("gamble", Named { name: "Roulette russe",
              desc: "Production ×{}, mais les inspecteurs gèlent {}× plus longtemps." }),
];

#[rustfmt::skip]
const SYNERGIES: &[(&str, Named)] = &[
    ("proliferation", Named { name: "Prolifération",
              desc: "Spores : +{} % de production par soldat au lieu de {} %." }),
    ("chain", Named { name: "Fission en cascade",
              desc: "Chaque détonation critique charge {} % de la masse critique." }),
    ("storm", Named { name: "Orage acide",
              desc: "La pluie acide ne divise plus la détonation." }),
    ("berserker", Named { name: "Berserker",
              desc: "Le combo retombe {} fois moins vite." }),
    ("smuggling", Named { name: "Réseau de contrebande",
              desc: "Les coffres proposent {} choix au lieu de {}." }),
    ("twins", Named { name: "Jumeaux maudits",
              desc: "Production ×{} supplémentaire." }),
];

#[rustfmt::skip]
const ACHIEVEMENTS: &[(&str, Named)] = &[
    ("boom", Named { name: "Premier boom", desc: "Déclenche l'explosion." }),
    ("lvl10", Named { name: "Ça chauffe", desc: "Niveau 10." }),
    ("lvl25", Named { name: "Petit nuage", desc: "Niveau 25." }),
    ("lvl50", Named { name: "Ça se voit de loin", desc: "Niveau 50." }),
    ("lvl100", Named { name: "Masse maximale", desc: "Niveau 100." }),
    ("lvl1k", Named { name: "Stratosphère", desc: "Niveau 1 000." }),
    ("lvl10k", Named { name: "Orbite basse", desc: "Niveau 10 000." }),
    ("lvl100k", Named { name: "Visible depuis Mars", desc: "Niveau 100 000." }),
    ("lvl1m", Named { name: "Le champignon ultime", desc: "Niveau 1 000 000." }),
    ("clicks100", Named { name: "Index musclé", desc: "100 appuis sur ESPACE." }),
    ("clicks1k", Named { name: "Tendinite", desc: "1 000 appuis sur ESPACE." }),
    ("clicks10k", Named { name: "Barre d'espace usée", desc: "10 000 appuis sur ESPACE." }),
    ("soldiers50", Named { name: "Section", desc: "50 soldats." }),
    ("soldiers100", Named { name: "Compagnie", desc: "100 soldats." }),
    ("tanks50", Named { name: "Division blindée", desc: "50 tanks." }),
    ("army", Named { name: "Armée complète", desc: "Au moins une unité de chaque type." }),
    ("units500", Named { name: "Mobilisation générale", desc: "500 unités au total." }),
    ("prod1k", Named { name: "Mille par seconde", desc: "1 000 ☢/s." }),
    ("prod1m", Named { name: "Mégatonne", desc: "1 M ☢/s." }),
    ("prod1b", Named { name: "Gigatonne", desc: "1 B ☢/s." }),
    ("rich", Named { name: "Réserve stratégique", desc: "1 T ☢ en stock." }),
    ("combo", Named { name: "Réaction en chaîne", desc: "Atteins le combo maximum." }),
    ("overheat", Named { name: "Doigts brûlés", desc: "Provoque une surchauffe." }),
    ("drop1", Named { name: "Colis reçu", desc: "Attrape un ravitaillement." }),
    ("drop25", Named { name: "Contrebandier", desc: "Attrape 25 ravitaillements." }),
    ("perfect", Named { name: "Timing parfait", desc: "Masse critique en zone verte." }),
    ("perfect10", Named { name: "Maître de la fission", desc: "10 masses critiques parfaites." }),
    ("inspector", Named { name: "Circulez !", desc: "Chasse un inspecteur de l'ONU." }),
    ("inspector25", Named { name: "Persona non grata", desc: "Chasse 25 inspecteurs." }),
    ("enemy1", Named { name: "Légitime défense", desc: "Repousse un ennemi (saboteur, drone...)." }),
    ("enemy25", Named { name: "Forteresse", desc: "Repousse 25 ennemis." }),
    ("frozen", Named { name: "Visite surprise", desc: "Laisse un inspecteur geler ta production." }),
    ("mutant", Named { name: "Mutant", desc: "Choisis 10 mutations." }),
    ("prestige1", Named { name: "Renaissance", desc: "Premier prestige." }),
    ("prestige10", Named { name: "Cycle éternel", desc: "10 prestiges." }),
    ("supernova", Named { name: "Supernova", desc: "Déclenche une supernova." }),
    ("synergy1", Named { name: "Alchimiste", desc: "Découvre une synergie de mutations." }),
    ("synergy_all", Named { name: "Généticien fou", desc: "Découvre toutes les synergies." }),
    ("speed", Named { name: "Éclair", desc: "Prestige en moins de 15 minutes." }),
    ("challenges", Named { name: "Vétéran des défis", desc: "Réussis 3 défis." }),
    ("challenges_all", Named { name: "Maître des défis", desc: "Réussis tous les défis." }),
    ("legend", Named { name: "Légende", desc: "Termine un défi au rang III." }),
    ("legend_all", Named { name: "Légende vivante", desc: "Termine tous les défis au rang III." }),
    ("double", Named { name: "Doublé", desc: "Réussis un double défi." }),
];

#[rustfmt::skip]
const ENEMIES: &[(&str, EnemyText)] = &[
    ("saboteur", EnemyText {
        name: "Saboteur",
        desc: "vole {} % de la radiation s'il atteint le champignon",
        news: "Un individu cagoulé avec un sac marqué « BUTIN » a été aperçu près du site.",
    }),
    ("drone", EnemyText {
        name: "Drone espion",
        desc: "vide la masse critique et appelle un inspecteur",
        news: "Un drone sans immatriculation survole la zone. Il a l'air de prendre des notes.",
    }),
    ("hippies", EnemyText {
        name: "Manifestants",
        desc: "production divisée par {} tant qu'ils sont là",
        news: "Des manifestants réclament la paix dans le monde et un champignon plus petit.",
    }),
    ("commando", EnemyText {
        name: "Commando",
        desc: "détruit {} % de chaque type d'unité s'il arrive",
        news: "Un commando en tenue de camouflage avance. Il est très mal camouflé devant un champignon \
                       orange.",
    }),
];

#[rustfmt::skip]
const NEWS: &[(u32, &str)] = &[
    (0, "Un militaire distrait a laissé une bombe dans le jardin. Rien d'inquiétant."),
    (0, "Les autorités recommandent de ne PAS appuyer sur la barre d'espace."),
    (1, "Le village voisin signale une légère hausse des températures."),
    (1, "Un agriculteur affirme que ses tomates brillent dans le noir."),
    (1, "Le maire inaugure un rond-point. Personne ne remarque le cratère juste à côté."),
    (2, "Un chat du voisinage a désormais trois queues. Il refuse de commenter."),
    (3, "Le ministère dément toute explosion et conseille de fermer les volets, par pure coquetterie."),
    (5, "Un réalisateur de série Z repère les lieux : « Pas besoin d'effets spéciaux. »"),
    (5, "Les soldats demandent des grenades plus grosses."),
    (10, "Les pigeons du quartier évitent désormais la zone."),
    (10, "La météo annonce un nuage en forme de champignon. « C'est normal », assure le ministère."),
    (12, "Tournage annulé pour « L'Attaque des tomates fluo » : les tomates ont attaqué pour de vrai."),
    (15, "Le compteur Geiger de la mairie est parti en vacances. Sans prévenir."),
    (20, "Un influenceur fait un selfie devant le champignon. Il ne s'est plus jamais coiffé."),
    (25, "Un acteur en costume de lézard géant postule comme mascotte du site. Refusé : pas assez crédible."),
    (30, "Les ventes de crème solaire indice 5000 explosent."),
    (35, "Les vaches du coin donnent du lait pétillant. Les fromagers sont perplexes."),
    (40, "Un champignonniste local porte plainte pour concurrence déloyale."),
    (45, "Sortie au cinéma de « Champignon 2 : le retour du retour ». Le public sort irradié d'émotion."),
    (50, "Le champignon est classé monument historique. Personne n'ose l'inspecter."),
    (60, "Les avions de ligne font désormais un détour de 300 km."),
    (70, "Un savant fou réclame des droits d'auteur sur le champignon. Il porte une blouse, c'est donc sérieux."),
    (75, "Une secte vénère le Grand Champignon. Ses membres ont tous le teint fluo."),
    (90, "Les scientifiques débattent : est-ce un champignon ou un nuage ? Les deux, apparemment."),
    (95, "Le doublage de la bande-annonce est confié à une voix grave qui dit « Cet été » toutes les trois secondes."),
    (100, "Le champignon atteint sa taille maximale. Il semble... frustré."),
    (200, "Les maquettes en carton du film catastrophe sont plus petites que le vrai champignon. Le tournage est reporté."),
    (300, "La NASA confirme : le champignon est visible depuis Mars."),
    (500, "Les Martiens envoient une soucoupe en fil de pêche pour voir de plus près."),
    (1_000, "Le champignon a son propre fuseau horaire."),
    (2_000, "Un festival de films de série Z organise sa cérémonie à l'ombre du champignon. Il fait très chaud."),
    (5_000, "Les extraterrestres demandent poliment de baisser le volume."),
    (10_000, "Le champignon reçoit un prix d'interprétation. Il n'a pas de discours, il n'a pas de bouche."),
    (20_000, "La Lune signale un léger réchauffement de sa face cachée."),
    (100_000, "Le champignon dépasse l'orbite des satellites. Les GPS indiquent tous « champignon »."),
    (200_000, "Les étoiles filantes font un détour."),
    (500_000, "Le Soleil se sent menacé."),
    (999_999, "Les astronomes renomment la Voie lactée « Le Grand Champignon »."),
];

const NEWS_PRESTIGE: &[&str] = &[
    "Déjà-vu : un militaire distrait laisse à nouveau une bombe dans le jardin.",
    "Les historiens notent que le champignon semble repousser de plus en plus vite.",
    "Suite, remake, reboot : le champignon est de retour, et il n'a rien appris.",
    "Le militaire distrait jure qu'on ne l'y reprendra plus. On l'y reprendra.",
];

const NEWS_NANAR: &[&str] = &[
    "Le champignon est en carton-pâte, mais le réalisateur jure que ça ne se voit pas.",
    "Une perche de micro apparaît dans le champ du champignon. Personne ne la retire.",
    "Faux raccord : le champignon a changé de couleur entre deux plans.",
    "Le budget effets spéciaux est épuisé. Les explosions seront désormais bruitées à la bouche.",
];
pub(super) const DATA: DataTexts = DataTexts {
    units: UNITS,
    unit_unlock: |unit| format!("Débloque l'unité : {unit}."),
    unit_unlock_after: |unit, challenge| {
        format!("Débloque l'unité : {unit}. Après le défi {challenge}.")
    },
    unit_mult: |unit, mult| format!("Production des {unit}s ×{mult} par rang."),
    talents: TALENTS,
    branches: BRANCHES,
    ptalents: PTALENTS,
    stalents: STALENTS,
    challenges: CHALLENGES,
    skins: SKINS,
    mutations: MUTATIONS,
    synergies: SYNERGIES,
    achievements: ACHIEVEMENTS,
    enemies: ENEMIES,
    chests: CHESTS,
    news: NEWS,
    news_prestige: NEWS_PRESTIGE,
    news_nanar: NEWS_NANAR,
    news_crocodile: "Un crocodile en caoutchouc sème la panique dans les rizières : le machiniste nie toute \
                     responsabilité.",
    news_inspector: "ALERTE : un inspecteur de l'ONU approche du site !",
};
