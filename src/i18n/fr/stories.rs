//! French introduction and challenge stories.

/// Introduction, shown before a new game (the first paragraph in bold).
pub(super) const INTRO: &[&str] = &[
    "ANNÉE 1962 ¾. QUELQUE PART ENTRE DEUX FRONTIÈRES. (STOCK-SHOT D'UN DÉSERT, PUIS D'UNE FORÊT, PUIS DU \
     MÊME DÉSERT.)",
    "Le professeur Amédée Morille, mycologue, artificier amateur et ceinture noire de pétanque, fait LA \
     découverte : les champignons atomiques sont VIVANTS ! Ils mangent de la radiation. « C'est \
     impossible ! », s'écrie son assistante, face à la mauvaise caméra.",
    "Une nuit d'orage (tournée en plein jour, filtre bleu), le professeur disparaît en zone interdite. On ne \
     retrouve que sa casquette et une bombe au fond de TON jardin, avec un post-it : « Arrose-la. »",
    "Te voici jardinier en chef. Le ministère envoie des soldats « pour aider », l'ONU des inspecteurs « pour \
     vérifier ». Saboteurs, drones en fil de pêche et manifestants débarquent sans invitation. Le voisin porte \
     plainte ; sa perruque change de couleur entre deux plans.",
    "Ta mission : le faire pousser jusqu'aux étoiles. Le professeur aurait été fier. Probablement : son \
     doublage était inaudible.",
    "CET ÉTÉ... APPUIE SUR ESPACE.",
];

/// Challenge stories, shown like the introduction at launch ("start") and on success ("end").
pub(super) const STORIES: &[((&str, &str), &[&str])] = &[
    (
        ("nanar", "start"),
        &[
            "NANARLAND — UN FILM DE GÉRARD ZÉDÉ",
            "Gérard Zédé, producteur des « Tomates tueuses du Larzac » et de « Mon voisin le yéti 3 : le retour \
             du yéti 2 », a vu ton champignon dans le journal. Il a eu une illumination. Et peut-être aussi une \
             légère insolation.",
            "Son prochain chef-d'œuvre s'appellera « LE CHAMPIGNON QUI VOULAIT CONQUÉRIR LE MONDE (ET UN PEU LA \
             BELGIQUE) ». Budget total : 312 francs, un sandwich au pâté et une caméra empruntée au beau-frère.",
            "Les explosions seront bruitées à la bouche par son neveu, qui est enrhumé : une fois sur trois, ça fait \
             pschitt. La moitié de ton armée est réquisitionnée comme figurants et ne produit plus que des \
             autographes.",
            "Ton rôle : faire monter le champignon jusqu'au niveau 100 avant la fin du tournage. Silence... \
             moteur... ça tourne ! (La perche est dans le champ. Personne ne la retirera.)",
        ],
    ),
    (
        ("nanar", "end"),
        &[
            "NANARLAND — GÉNÉRIQUE DE FIN",
            "Le film sort un mardi pluvieux dans la salle des fêtes de Saint-Glinglin-sur-Marne. Quatorze \
             spectateurs. À la scène la plus dramatique, les quatorze pleurent de rire. Le lendemain, ils \
             reviennent avec leurs amis.",
            "Les critiques sont unanimes : « raté de A à Z », « le perchman mérite un César », « on y retourne \
             demain ». Six mois plus tard, la séance de minuit affiche complet tous les vendredis, et la salle \
             entière crie « PSCHITT ! » à chaque explosion. Ce n'est pas un navet : c'est un nanar, et on l'aime.",
            "Ton champignon est devenu CULTE : ses détonations critiques frappent désormais ×15. Gérard t'offre \
             les décors en carton-pâte, à porter comme skin.",
            "Tout en bas du générique, une ligne intrigue les fans : « Lézard géant : A. Morille ». Le professeur \
             ne répond toujours pas aux lettres.",
        ],
    ),
    (
        ("mono", "start"),
        &[
            "MONOCULTURE — UNE DIRECTIVE DU MINISTÈRE",
            "Le ministère de l'Agriculture Militaire a envoyé un inspecteur des récoltes. Costume beige, moustache \
             réglementaire, tampon encreur à la ceinture. Il compte tes troupes, fronce les sourcils et sort un \
             formulaire en sept exemplaires.",
            "« Trop de variétés ! On ne mélange pas les soldats et les mortiers, pas plus que les carottes et les \
             navets. » Désormais, ton champ n'aura qu'UNE seule culture : la première que tu achèteras. Les \
             vétérans, eux, sont priés de rester chez eux : ils n'ont pas rempli le bon formulaire.",
            "Ta mission : atteindre le niveau demandé avec une armée en rangs parfaitement identiques. Le ministère \
             appelle ça « l'harmonie des sillons ». Les soldats appellent ça « l'ennui ».",
        ],
    ),
    (
        ("mono", "end"),
        &[
            "MONOCULTURE — RAPPORT DE RÉCOLTE",
            "L'inspecteur repasse, tampon levé. Il cherche une irrégularité, n'importe laquelle, jusque sous les \
             casques. Rien. Tes troupes sont si bien alignées qu'on pourrait les labourer.",
            "Ébloui, il te décerne la médaille du Rendement Agricole (catégorie militaire) et une méthode secrète \
             du ministère : à chaque palier, tes unités produisent un peu plus qu'avant. Tu gagnes aussi un skin \
             Mousse, couleur officielle des pelouses ministérielles.",
            "En partant, il glisse sur une peau de banane. Une seule. Monoculture, on t'avait dit.",
        ],
    ),
    (
        ("halflife", "start"),
        &[
            "DEMI-VIE — UN COURRIER DU LABORATOIRE",
            "Le laboratoire du professeur Morille a enfin livré l'uranium commandé en 1958. Il est arrivé par la \
             poste, dans un colis marqué « FRAGILE » et « NE PAS LÉCHER ». Petit problème : la date de péremption \
             est dépassée depuis longtemps.",
            "Ta radiation s'évapore à vue d'œil : la moitié disparaît en quelques secondes, et ta progression vers le \
             niveau suivant aussi. Pire, tes troupes se désintègrent peu à peu. Les petites escouades tiennent bon, \
             les grosses armées fondent comme neige au soleil.",
            "Ta mission : atteindre le niveau demandé avant que tout ne parte en fumée. Dépense vite, rachète \
             souvent, et ne garde rien pour demain.",
        ],
    ),
    (
        ("halflife", "end"),
        &[
            "DEMI-VIE — RAPPORT DE DÉSINTÉGRATION",
            "Contre toute attente, tu as tenu. Les physiciens du monde entier s'interrogent : comment un champignon \
             peut-il grandir plus vite qu'il ne se désintègre ? Un prix Nobel est envisagé, puis annulé faute de \
             candidat capable de rester entier jusqu'à la cérémonie.",
            "Tes soldats ont appris à enchaîner les gestes sans perdre une seconde : ton combo max augmente. Tu \
             gardes aussi le colis d'uranium, recyclé en skin Uranium. Il brille toujours. Ne pas lécher.",
        ],
    ),
    (
        ("strike", "start"),
        &[
            "GRÈVE GÉNÉRALE — COMMUNIQUÉ DU SYNDICAT DES TROUFIONS",
            "Le Syndicat Unifié des Troufions Atomiques (SUTA) a voté à main levée, puis à main baissée, puis à main \
             levée à nouveau, pour être sûr. Résultat : GRÈVE GÉNÉRALE jusqu'à nouvel ordre.",
            "Leurs revendications : des casques à leur taille, une cantine sans soupe aux champignons, et surtout \
             de la RECONNAISSANCE. Ils ne travailleront que quand ils verront le chef au boulot. À la seconde où tu \
             lâches le détonateur, ils posent les outils. L'auto-détonateur ne les trompe pas : « C'est un robot, \
             ça ne compte pas. »",
            "Ta mission : atteindre le niveau demandé en montrant l'exemple. Appuie sans relâche. Le piquet de \
             grève te surveille.",
        ],
    ),
    (
        ("strike", "end"),
        &[
            "GRÈVE GÉNÉRALE — FIN DU CONFLIT",
            "Les négociations aboutissent tard dans la nuit, autour d'un café tiède et d'un paquet de biscuits secs. \
             Le syndicat obtient des casques à sa taille. Tu obtiens la paix sociale et une poignée de main si ferme \
             qu'elle a laissé une marque.",
            "Galvanisées par ton exemple, tes troupes mettent du cœur à l'ouvrage à chaque appui : l'onde de choc \
             porte deux fois plus loin. Le syndicat t'offre sa banderole, reconvertie en skin Crépuscule.",
            "Le délégué syndical, ému, te nomme membre d'honneur. Tu devras maintenant faire grève contre toi-même. \
             Le règlement est formel.",
        ],
    ),
    (
        ("moussaka", "start"),
        &[
            "L'ATTAQUE DE LA MOUSSAKA GÉANTE — UN REMAKE DE GÉRARD ZÉDÉ",
            "Gérard Zédé est de retour. Il a revu un chef-d'œuvre méconnu du cinéma grec de 1999, en a pleuré deux \
             nuits, et veut en tourner le remake chez toi. Il manquait un effet spécial : ton champignon atomique \
             a irradié le gratin de la cantine.",
            "Le plat a mal tourné. Ou plutôt, il a très bien tourné : aubergines, viande hachée, trois kilos de \
             béchamel, et une faim insatiable. La MOUSSAKA GÉANTE avance dans la ville et grossit de minute en \
             minute. Plus elle grossit, plus elle avale ta radiation.",
            "Ta mission : atteindre le niveau demandé avant qu'elle ne recouvre tout. Le neveu enrhumé fait les \
             bruits de mastication. Le perchman, lui, a été englouti dès la première prise.",
        ],
    ),
    (
        ("moussaka", "end"),
        &[
            "L'ATTAQUE DE LA MOUSSAKA GÉANTE — GÉNÉRIQUE DE FIN",
            "Repue, la moussaka s'allonge sur la place du marché et pousse un soupir de béchamel. Elle ne veut plus \
             rien dévorer, juste qu'on lui gratte la croûte. Gérard Zédé crie « coupez ! » puis « on garde ! ».",
            "Elle s'engage dans ton armée sous le nom de Moussaka apprivoisée : une unité qui produit plus qu'un \
             trou noir et sent nettement meilleur. Tu gagnes aussi le skin Moussaka géante, gratiné à souhait.",
            "Le perchman est retrouvé sous la troisième couche, vivant et rassasié. Il ne veut plus entendre parler \
             de cinéma. Il ouvre une taverne.",
        ],
    ),
    (
        ("savior", "start"),
        &[
            "L'HOMME QUI SAUVE LE MONDE — UN REMAKE DE GÉRARD ZÉDÉ",
            "Gérard Zédé a déniché une cassette turque de 1982, jamais sortie en France, que la rédaction d'un \
             certain site place tout en haut de son classement. Il l'appelle « le Saint Graal ». Il veut le même, \
             chez toi, tout de suite.",
            "Le héros moustachu de son remake ne délègue rien. Ton armée attend les bras croisés : elle ne crache \
             sa radiation que quand le héros cogne. Et il cogne sur ESPACE, encore et encore, comme sur un rocher \
             d'entraînement.",
            "Ta mission : atteindre le niveau demandé à la force du poignet. Le monde ne va pas se sauver tout \
             seul.",
        ],
    ),
    (
        ("savior", "end"),
        &[
            "L'HOMME QUI SAUVE LE MONDE — GÉNÉRIQUE DE FIN",
            "Le monde est sauvé. Probablement. Le montage est si rapide que personne n'a bien vu. Gérard Zédé \
             pleure dans les bras du perchman, qui commence à avoir l'habitude.",
            "Le héros moustachu reste dans ton armée : le Sauveur du monde frappe désormais pour toi, et sa \
             production double à chaque rang du défi. Tu gagnes aussi le skin Sauveur du monde, jaune et rouge \
             comme une affiche de vidéoclub.",
        ],
    ),
    (
        ("whitefire", "start"),
        &[
            "WHITE FIRE — UN REMAKE DE GÉRARD ZÉDÉ",
            "Trois titres, un film, un monument : White Fire, alias Vivre pour survivre, alias Le diamant. \
             Gérard Zédé en a fait son livre de chevet. Pour le remake, il a réuni des gros bras et un diamant \
             radioactif plus gros qu'un melon.",
            "Problème : sur ce tournage, la moindre réplique tourne à la baston homérique. Chaque appui sur ESPACE \
             risque de déclencher une bagarre générale qui fait voler une partie de ta radiation à travers le \
             décor.",
            "Ta mission : atteindre le niveau demandé avant que l'équipe ne casse tout. Le cascadeur en chef a \
             déjà demandé une prime.",
        ],
    ),
    (
        ("whitefire", "end"),
        &[
            "WHITE FIRE — GÉNÉRIQUE DE FIN",
            "La dernière baston s'achève. Personne ne sait plus qui se battait contre qui, ni pourquoi. Le diamant, \
             lui, brille toujours, intact, au fond d'un seau.",
            "Tu le gardes : le Diamant blanc rejoint ton armée et irradie de plus belle, deux fois plus à chaque \
             rang du défi. Le skin Feu blanc te revient aussi. Le film sort sous trois titres différents ; tu \
             achètes les trois cassettes.",
        ],
    ),
    (
        ("crocodile", "start"),
        &[
            "CROCODILE FURY — UN REMAKE DE GÉRARD ZÉDÉ",
            "Hong Kong, 1988 : deux films collés ensemble, des vampires d'un côté, un crocodile de l'autre, et un \
             doublage improvisé après le tournage. Gérard Zédé trouve ça génial. Pour son remake, il a acheté un \
             crocodile en caoutchouc d'occasion.",
            "Le crocodile a faim. Toutes les trente secondes, il surgit de la rizière et dévore une partie d'une \
             sorte de troupes au hasard. Il paraît que c'est la réincarnation de la fiancée du héros. Elle n'a \
             pas bon caractère.",
            "Ta mission : atteindre le niveau demandé en remplaçant les disparus plus vite qu'il ne les mange.",
        ],
    ),
    (
        ("crocodile", "end"),
        &[
            "CROCODILE FURY — GÉNÉRIQUE DE FIN",
            "Le héros serre le crocodile en caoutchouc dans ses bras : « Je comprends ce que tu ressens, mais tu \
             ne peux pas continuer à dévorer tous mes soldats innocents. » Le crocodile hoche la tête. Le \
             machiniste qui le tient aussi.",
            "Apaisé, il s'engage : le Croco en caoutchouc produit pour toi, deux fois plus à chaque rang du défi. \
             Tu gagnes le skin Rizière. La moitié du film reste à monter ; Gérard dit que ça fera une suite.",
        ],
    ),
    (
        ("devil", "start"),
        &[
            "DEVIL STORY — UN REMAKE DE GÉRARD ZÉDÉ",
            "Normandie, 1985 : un village hanté, une falaise en maquette de bateau, une momie sans sarcophage et \
             un cheval noir qui revient sans cesse à l'écran. Gérard Zédé n'en revient pas. Il tourne le remake \
             sur place, avec le même cheval. Enfin, la même cassette.",
            "Le monteur n'a qu'un seul plan du cheval noir, et il le remet entre chaque scène. Résultat : impossible \
             d'enchaîner les achats. Une seule unité à la fois, puis le cheval repasse.",
            "Ta mission : atteindre le niveau demandé malgré le stock-shot. Un vieux tire au fusil sans jamais \
             recharger, de minuit à l'aube. Personne ne sait sur quoi.",
        ],
    ),
    (
        ("devil", "end"),
        &[
            "DEVIL STORY — GÉNÉRIQUE DE FIN",
            "La momie sort enfin de son couvercle et retrouve sa fiancée. Le faux sang gicle à la poire pendant \
             une bonne minute. Puis tout s'avère n'être qu'un rêve, et le chat noir du début vient regarder la \
             fin.",
            "La momie reste : la Momie de la falaise rejoint ton armée et produit deux fois plus à chaque rang du \
             défi. Tu gagnes le skin Gore-mandie. Le cheval noir, lui, repasse une dernière fois. Pour la route.",
        ],
    ),
    (
        ("ninja", "start"),
        &[
            "BLACK NINJA — UN REMAKE DE GÉRARD ZÉDÉ",
            "Hong Kong, 1987 : deux policiers d'Interpol, un film taïwanais cousu au milieu, une fausse France \
             faite de baguettes et de bouteilles d'eau, et des ninjas qui se battent en accéléré. Titre original : \
             Ninja: Silent Assassin. Gérard Zédé a pris des notes.",
            "Sur ce tournage, le ninja frappe en silence : pas de combo, pas de crescendo. Chaque appui sur ESPACE \
             compte pour un seul coup. Les moulinets de bras, eux, sont autorisés.",
            "Ta mission : atteindre le niveau demandé, coup par coup. Une pastèque a été prévue pour chaque prise. \
             Le budget pastèques dépasse celui des cascades.",
        ],
    ),
    (
        ("ninja", "end"),
        &[
            "BLACK NINJA — GÉNÉRIQUE DE FIN",
            "Le dernier trafiquant tombe, en accéléré. La musique ressemble furieusement à celle d'une série \
             policière américaine. Les avocats de Gérard Zédé sont formels : c'est une coïncidence.",
            "Le ninja reste à ton service : le Ninja d'Interpol produit deux fois plus à chaque rang du défi. Tu \
             gagnes le skin Black Carambar. Il ne reste aucune pastèque intacte sur le plateau.",
        ],
    ),
    (
        ("pacifist", "start"),
        &[
            "PACIFISTE — UNE PÉTITION DU VOISINAGE",
            "Le comité des fêtes a fait signer une pétition : plus de soldats dans le jardin, ça abîme les \
             hortensias. Trois cents signatures, dont celle du chat de la voisine, tamponnée avec la patte.",
            "Ton armée est donc renvoyée à la maison, avec un sandwich et un bon de réduction pour le musée de la \
             Paix. Il ne te reste que le détonateur, ton index et ta détermination.",
            "Ta mission : faire grandir le champignon à la seule force du doigt. Les hortensias te regardent.",
        ],
    ),
    (
        ("pacifist", "end"),
        &[
            "PACIFISTE — LA COLOMBE ET LE CHAMPIGNON",
            "Le champignon a grandi sans un seul soldat. Le comité des fêtes en fait la mascotte de la kermesse ; \
             une colombe vient même faire son nid dans la fumée, ce qui pose des questions sanitaires.",
            "Ton index, devenu légendaire, frappe désormais plus fort à chaque détonation. Tu gagnes aussi le skin \
             Vert radioactif, couleur officielle des hortensias depuis l'incident.",
        ],
    ),
    (
        ("onearm", "start"),
        &[
            "MANCHOT — UN ACCIDENT DE PÉTANQUE",
            "Le tournoi de pétanque du professeur Morille a mal tourné : ton bras droit est dans le plâtre, ton bras \
             gauche tient le pastis. Impossible d'appuyer sur ESPACE. L'auto-détonateur, jaloux, refuse de \
             travailler sans toi.",
            "Le ministère t'envoie trois soldats « pour le principe ». Ils ont lu le manuel. Le reste, c'est à \
             l'armée de le faire toute seule.",
            "Ta mission : atteindre le niveau demandé sans toucher au détonateur. Le plâtre est signé par tout le \
             régiment.",
        ],
    ),
    (
        ("onearm", "end"),
        &[
            "MANCHOT — LE PLÂTRE TOMBE",
            "Le médecin découpe le plâtre : ton bras est guéri, et l'armée a appris à se débrouiller sans toi. Elle \
             en est très fière ; elle en parle à chaque repas.",
            "Cette autonomie lui reste : la production de toute l'armée augmente à chaque rang du défi. Le plâtre, \
             couvert de dessins, devient le skin Arc-en-ciel.",
        ],
    ),
    (
        ("race", "start"),
        &[
            "COURSE — LE GRAND PRIX DU CHAMPIGNON",
            "Un organisateur de rallye a vu ton champignon depuis l'autoroute. Il veut l'inscrire au Grand Prix des \
             Phénomènes Inexpliqués : départ immédiat, arrivée au niveau demandé, chronomètre en main.",
            "Le règlement est simple : si le temps est écoulé avant la ligne d'arrivée, le champignon est \
             disqualifié et doit rentrer à pied. Le drapeau à damier est déjà en position.",
            "Ta mission : aller vite. Très vite. Le public a apporté des sandwichs mais pas de patience.",
        ],
    ),
    (
        ("race", "end"),
        &[
            "COURSE — SUR LE PODIUM",
            "Ligne d'arrivée franchie ! Le champignon monte sur le podium, où il ne tient pas. On l'arrose de \
             champagne, qui s'évapore instantanément. La foule acclame un nuage.",
            "En récompense, les isotopes gagnés à chaque prestige augmentent à chaque rang. Tu gagnes aussi le skin \
             Champignon de Mario, coiffé comme un pilote. Les avocats d'un plombier moustachu ont été prévenus.",
        ],
    ),
    (
        ("inflation", "start"),
        &[
            "INFLATION — LA CRISE",
            "Les marchés s'affolent : le prix du soldat a été multiplié par quatre pendant la nuit. Les économistes \
             accusent le champignon, le champignon accuse les économistes, et le boulanger augmente la baguette \
             par solidarité.",
            "Chaque unité coûte une fortune. Le ministère te conseille de « faire des efforts », puis augmente ses \
             propres honoraires.",
            "Ta mission : atteindre le niveau demandé avec un budget en carton. Compte bien la monnaie.",
        ],
    ),
    (
        ("inflation", "end"),
        &[
            "INFLATION — FIN DE LA CRISE",
            "La crise est finie : les prix redescendent, les économistes trouvent un autre coupable, et le \
             boulanger garde ses nouveaux prix « par habitude ».",
            "Toi, tu as appris à négocier : tes unités coûtent moins cher à chaque rang du défi. Ta cagnotte, \
             fondue en lingot, devient le skin Or massif.",
        ],
    ),
    (
        ("blind", "start"),
        &[
            "BROUILLARD DE GUERRE — UNE NUIT SANS LUNE",
            "Une purée de pois tombe sur le jardin. On n'y voit plus rien : ni le niveau, ni la radiation, ni la \
             production. Les soldats se perdent entre le potager et la cabane à outils.",
            "Les instruments de mesure ont tous été prêtés au voisin, qui ne les rendra pas. Il faudra jouer à \
             l'instinct, à l'oreille, et au flair.",
            "Ta mission : atteindre le niveau demandé à l'aveugle. Tu le sauras quand tu y seras.",
        ],
    ),
    (
        ("blind", "end"),
        &[
            "BROUILLARD DE GUERRE — LE BROUILLARD SE LÈVE",
            "Le soleil revient. Le champignon est là, plus grand que jamais, entouré de soldats qui se demandent \
             depuis combien de temps ils montent la garde devant la mauvaise cabane.",
            "Ton instinct s'est aiguisé : on te propose une mutation de plus à chaque rang du défi. Un soldat \
             resté dans la brume, devenu translucide, te donne le skin Fantôme.",
        ],
    ),
    (
        ("watched", "start"),
        &[
            "SOUS SURVEILLANCE — UNE VISITE DE L'ONU",
            "L'ONU a envoyé toute une délégation. Un inspecteur arrive, puis un autre, puis un troisième, chacun \
             avec un formulaire différent et le même imperméable beige.",
            "Les gardes ont été mis en congé « pour transparence ». Les brouillards de mutation ne trompent plus \
             personne. Il faudra chasser chaque inspecteur toi-même, et vite.",
            "Ta mission : atteindre le niveau demandé malgré le défilé. Garde une main sur X.",
        ],
    ),
    (
        ("watched", "end"),
        &[
            "SOUS SURVEILLANCE — RAPPORT FINAL",
            "La délégation repart avec un rapport de quatre cents pages qui conclut, en substance, que « c'est un \
             champignon ». Une copie est classée, une autre sert à caler une table.",
            "Tu as l'habitude, maintenant : chaque inspecteur chassé laisse plus de butin à chaque rang du défi. \
             Les gyrophares de la délégation deviennent le skin Néon.",
        ],
    ),
    (
        ("ascete", "start"),
        &[
            "ASCÈTE — UNE RETRAITE SPIRITUELLE",
            "Un moine en tournée a convaincu ton champignon de renoncer au superflu. Plus de détonateur renforcé, \
             plus de propagande, plus de marché noir : seulement l'essentiel, c'est-à-dire de nouvelles unités.",
            "Les talents de confort sont rangés au grenier. Le champignon médite, l'armée aussi, et le sergent \
             fait du yoga en treillis.",
            "Ta mission : atteindre le niveau demandé avec le strict minimum. Respire. Recrute.",
        ],
    ),
    (
        ("ascete", "end"),
        &[
            "ASCÈTE — L'ILLUMINATION",
            "Le champignon atteint l'illumination, ou peut-être juste le niveau demandé. Le moine repart, satisfait, \
             avec un souvenir radioactif dans son baluchon.",
            "La sagesse reste : tu commences chaque run avec plus de PT à chaque rang du défi. Le calme glacial du \
             monastère t'offre le skin Glace.",
        ],
    ),
    (
        ("hothead", "start"),
        &[
            "TÊTE BRÛLÉE — CANICULE",
            "Quarante-deux degrés à l'ombre, et il n'y a pas d'ombre. Le détonateur fond à moitié et surchauffe \
             au moindre effort ; même les Doigts d'amiante transpirent.",
            "Le moindre emballement le bloque de longues secondes. Il faudra appuyer en mesure, comme un métronome \
             en vacances.",
            "Ta mission : atteindre le niveau demandé sans griller le détonateur. Un ventilateur est en route. Il \
             est en panne.",
        ],
    ),
    (
        ("hothead", "end"),
        &[
            "TÊTE BRÛLÉE — L'ORAGE",
            "Un orage éclate enfin. Le détonateur fume, siffle, puis refroidit. Tes doigts, eux, ont gagné en \
             endurance.",
            "Le seuil de surchauffe monte à chaque rang du défi. Le détonateur, à moitié fondu, garde une jolie \
             couleur : le skin Lave.",
        ],
    ),
    (
        ("invasion", "start"),
        &[
            "INVASION — TOUS EN MÊME TEMPS",
            "Saboteurs, drones, manifestants et commandos ont reçu le même carton d'invitation, avec la même date. \
             Ils arrivent tous dès le début, et ils ne viennent pas pour le buffet.",
            "Un ennemi débarque toutes les quelques secondes. Le jardin ressemble à un parc d'attractions un jour \
             de grève des contrôleurs.",
            "Ta mission : atteindre le niveau demandé en tenant le siège. Martèle X, et ne lâche rien.",
        ],
    ),
    (
        ("invasion", "end"),
        &[
            "INVASION — LE SIÈGE EST LEVÉ",
            "Le dernier commando repart en boitant. Les manifestants plient leurs banderoles, les drones rentrent \
             au garage, et le saboteur oublie sa clé à molette.",
            "Les ennemis marchent désormais plus lentement à chaque rang du défi : ils se souviennent. Le treillis \
             du commando, abandonné dans la haie, devient le skin Camouflage.",
        ],
    ),
    (
        ("sterile", "start"),
        &[
            "STÉRILE — L'HYGIÈNE AVANT TOUT",
            "Un inspecteur sanitaire a désinfecté tout le jardin à l'eau de Javel. Plus une spore, plus une \
             mutation : le champignon est propre comme un sou neuf, et un peu vexé.",
            "Aucune mutation ne te sera proposée. Il faudra grandir à l'ancienne, sans tentacules ni yeux en trop.",
            "Ta mission : atteindre le niveau demandé sans le moindre coup de pouce génétique.",
        ],
    ),
    (
        ("sterile", "end"),
        &[
            "STÉRILE — RETOUR À LA NATURE",
            "Les spores reviennent au premier coup de vent. Le champignon, qui a appris à s'en passer, les accueille \
             avec une certaine condescendance.",
            "Sa masse critique se charge plus vite à chaque rang du défi. Le carrelage javellisé, pur et brillant, \
             devient le skin Cristal.",
        ],
    ),
    (
        ("unlucky", "start"),
        &[
            "CHAT NOIR — VENDREDI 13",
            "Un chat noir a traversé le jardin sous une échelle, en cassant un miroir. Depuis, plus rien ne marche : \
             pas de détonation critique, pas de masse critique, pas un seul ravitaillement.",
            "L'avion de ravitaillement fait demi-tour au-dessus de la maison. Le pilote dit qu'il « le sent mal ».",
            "Ta mission : atteindre le niveau demandé sans la moindre chance. Le chat, lui, va très bien.",
        ],
    ),
    (
        ("unlucky", "end"),
        &[
            "CHAT NOIR — LA CHANCE TOURNE",
            "Le chat noir s'endort au pied du champignon, au chaud. La malchance se lève comme un brouillard du \
             matin, et un trèfle à quatre feuilles pousse dans la pelouse irradiée.",
            "Tes détonations deviennent critiques plus souvent à chaque rang du défi. Le pelage du chat t'inspire le \
             skin Obsidienne.",
        ],
    ),
];
