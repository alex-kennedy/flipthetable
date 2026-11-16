use bevy::prelude::*;

use crate::pixel_art::ProceduralTextures;
use crate::scenario_data::PropPrefab;

/// Pre-allocated GPU materials for all runtime VFX, decals, cracked states, and shards.
/// Prevents any runtime `Assets<StandardMaterial>::add` stalls or WebGL bind-group churn during mass explosions!
#[derive(Resource, Clone)]
pub struct VfxPalette {
    pub spark_cyan: Handle<StandardMaterial>,
    pub spark_gold: Handle<StandardMaterial>,
    pub smoke: Handle<StandardMaterial>,
    pub fire_core: Handle<StandardMaterial>,
    pub fire_flame: Handle<StandardMaterial>,
    pub foam_white: Handle<StandardMaterial>,
    pub steam_wisp: Handle<StandardMaterial>,
    pub hiss_jet: Handle<StandardMaterial>,
    pub dust_tan: Handle<StandardMaterial>,
    pub dust_grey: Handle<StandardMaterial>,

    pub cracked_screen: Handle<StandardMaterial>,
    pub spiderweb_crack: Handle<StandardMaterial>,
    pub scorch_decal: Handle<StandardMaterial>,
    pub sheet_paper: Handle<StandardMaterial>,
    pub sheet_blueprint: Handle<StandardMaterial>,

    pub puddle_coffee: Handle<StandardMaterial>,
    pub puddle_water: Handle<StandardMaterial>,
    pub puddle_scotch: Handle<StandardMaterial>,
    pub puddle_paint_blue: Handle<StandardMaterial>,
    pub puddle_paint_red: Handle<StandardMaterial>,
    pub puddle_paint_yellow: Handle<StandardMaterial>,

    pub drop_coffee: Handle<StandardMaterial>,
    pub drop_water: Handle<StandardMaterial>,
    pub drop_scotch: Handle<StandardMaterial>,
    pub drop_paint_blue: Handle<StandardMaterial>,
    pub drop_paint_red: Handle<StandardMaterial>,
    pub drop_paint_yellow: Handle<StandardMaterial>,

    pub shard_pcb: Handle<StandardMaterial>,
    pub shard_phosphor: Handle<StandardMaterial>,
    pub shard_glass: Handle<StandardMaterial>,
    pub shard_beige: Handle<StandardMaterial>,
    pub shard_dark_metal: Handle<StandardMaterial>,
    pub shard_silver: Handle<StandardMaterial>,
    pub shard_olive: Handle<StandardMaterial>,
    pub shard_wood: Handle<StandardMaterial>,
    pub shard_white: Handle<StandardMaterial>,
    pub shard_red: Handle<StandardMaterial>,
    pub shard_yellow: Handle<StandardMaterial>,
    pub shard_green: Handle<StandardMaterial>,
}

impl VfxPalette {
    pub fn new(materials: &mut Assets<StandardMaterial>, textures: &ProceduralTextures) -> Self {
        let mk_puddle = |mats: &mut Assets<StandardMaterial>, col: Color| {
            mats.add(StandardMaterial {
                base_color: col,
                base_color_texture: Some(textures.splatter_decal.clone()),
                alpha_mode: AlphaMode::Mask(0.45),
                perceptual_roughness: 0.22,
                ..default()
            })
        };
        let mk_drop = |mats: &mut Assets<StandardMaterial>, col: Color| {
            mats.add(StandardMaterial {
                base_color: col,
                emissive: col.to_linear() * 0.18,
                perceptual_roughness: 0.18,
                ..default()
            })
        };
        let mk_shard = |mats: &mut Assets<StandardMaterial>, col: Color, rough: f32| {
            mats.add(StandardMaterial {
                base_color: col,
                perceptual_roughness: rough,
                ..default()
            })
        };

        let coffee_c = Color::srgb(0.25, 0.12, 0.06);
        let water_c = Color::srgb(0.22, 0.68, 0.96);
        let scotch_c = Color::srgb(0.78, 0.40, 0.09);
        let p_blue = Color::srgb(0.18, 0.58, 0.98);
        let p_red = Color::srgb(0.96, 0.24, 0.20);
        let p_yel = Color::srgb(0.98, 0.80, 0.14);

        Self {
            spark_cyan: materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.98, 1.0),
                emissive: LinearRgba::rgb(2.6, 3.4, 3.8),
                unlit: true,
                ..default()
            }),
            spark_gold: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.90, 0.32),
                emissive: LinearRgba::rgb(3.4, 2.4, 0.45),
                unlit: true,
                ..default()
            }),
            smoke: materials.add(StandardMaterial {
                base_color: Color::srgba(0.18, 0.20, 0.25, 0.78),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            fire_core: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.96, 0.55),
                emissive: LinearRgba::rgb(4.0, 3.0, 0.85),
                unlit: true,
                ..default()
            }),
            fire_flame: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.42, 0.10),
                emissive: LinearRgba::rgb(3.0, 0.90, 0.15),
                unlit: true,
                ..default()
            }),
            foam_white: materials.add(StandardMaterial {
                base_color: Color::srgb(0.94, 0.96, 0.99),
                emissive: LinearRgba::rgb(0.45, 0.50, 0.55),
                unlit: true,
                ..default()
            }),
            steam_wisp: materials.add(StandardMaterial {
                base_color: Color::srgba(0.95, 0.94, 0.90, 0.36),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            hiss_jet: materials.add(StandardMaterial {
                base_color: Color::srgba(0.95, 0.97, 1.0, 0.72),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            dust_tan: materials.add(StandardMaterial {
                base_color: Color::srgba(0.76, 0.68, 0.56, 0.70),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            dust_grey: materials.add(StandardMaterial {
                base_color: Color::srgba(0.68, 0.72, 0.78, 0.70),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),

            cracked_screen: materials.add(StandardMaterial {
                base_color_texture: Some(textures.crt_cracked.clone()),
                emissive: LinearRgba::rgb(0.95, 1.35, 1.45),
                emissive_texture: Some(textures.crt_cracked.clone()),
                ..default()
            }),
            spiderweb_crack: materials.add(StandardMaterial {
                base_color_texture: Some(textures.spiderweb_crack.clone()),
                alpha_mode: AlphaMode::Mask(0.45),
                unlit: true,
                ..default()
            }),
            scorch_decal: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(textures.scorch_decal.clone()),
                alpha_mode: AlphaMode::Mask(0.45),
                perceptual_roughness: 0.9,
                ..default()
            }),
            sheet_paper: materials.add(StandardMaterial {
                base_color_texture: Some(textures.printed_page.clone()),
                cull_mode: None,
                perceptual_roughness: 0.88,
                ..default()
            }),
            sheet_blueprint: materials.add(StandardMaterial {
                base_color_texture: Some(textures.blueprint_page.clone()),
                cull_mode: None,
                perceptual_roughness: 0.88,
                ..default()
            }),

            puddle_coffee: mk_puddle(materials, coffee_c),
            puddle_water: mk_puddle(materials, water_c),
            puddle_scotch: mk_puddle(materials, scotch_c),
            puddle_paint_blue: mk_puddle(materials, p_blue),
            puddle_paint_red: mk_puddle(materials, p_red),
            puddle_paint_yellow: mk_puddle(materials, p_yel),

            drop_coffee: mk_drop(materials, coffee_c),
            drop_water: mk_drop(materials, water_c),
            drop_scotch: mk_drop(materials, scotch_c),
            drop_paint_blue: mk_drop(materials, p_blue),
            drop_paint_red: mk_drop(materials, p_red),
            drop_paint_yellow: mk_drop(materials, p_yel),

            shard_pcb: mk_shard(materials, Color::srgb(0.12, 0.52, 0.26), 0.45),
            shard_phosphor: materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.92, 0.86),
                emissive: LinearRgba::rgb(0.4, 1.3, 1.2),
                ..default()
            }),
            shard_glass: materials.add(StandardMaterial {
                base_color: Color::srgba(0.76, 0.93, 1.0, 0.78),
                emissive: LinearRgba::rgb(0.22, 0.40, 0.50),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.12,
                ..default()
            }),
            shard_beige: mk_shard(materials, Color::srgb(0.84, 0.79, 0.67), 0.65),
            shard_dark_metal: mk_shard(materials, Color::srgb(0.18, 0.21, 0.27), 0.50),
            shard_silver: mk_shard(materials, Color::srgb(0.75, 0.79, 0.84), 0.38),
            shard_olive: mk_shard(materials, Color::srgb(0.36, 0.41, 0.28), 0.60),
            shard_wood: mk_shard(materials, Color::srgb(0.54, 0.32, 0.17), 0.68),
            shard_white: mk_shard(materials, Color::srgb(0.91, 0.92, 0.94), 0.45),
            shard_red: mk_shard(materials, Color::srgb(0.86, 0.18, 0.18), 0.42),
            shard_yellow: mk_shard(materials, Color::srgb(0.96, 0.76, 0.14), 0.42),
            shard_green: mk_shard(materials, Color::srgb(0.20, 0.62, 0.32), 0.68),
        }
    }

    pub fn liquid_handles(&self, col: Color) -> (Handle<StandardMaterial>, Handle<StandardMaterial>) {
        let lin = col.to_linear();
        if lin.blue > 0.6 && lin.green > 0.25 {
            if lin.green > 0.35 {
                (self.drop_water.clone(), self.puddle_water.clone())
            } else {
                (self.drop_paint_blue.clone(), self.puddle_paint_blue.clone())
            }
        } else if lin.red > 0.7 && lin.green > 0.45 {
            (self.drop_paint_yellow.clone(), self.puddle_paint_yellow.clone())
        } else if lin.red > 0.7 && lin.green < 0.15 {
            (self.drop_paint_red.clone(), self.puddle_paint_red.clone())
        } else if lin.red > 0.35 {
            (self.drop_scotch.clone(), self.puddle_scotch.clone())
        } else {
            (self.drop_coffee.clone(), self.puddle_coffee.clone())
        }
    }

    pub fn shard_handle_for(&self, col: Color) -> Handle<StandardMaterial> {
        let lin = col.to_linear();
        if lin.red > 0.75 && lin.green > 0.75 && lin.blue > 0.75 {
            self.shard_white.clone()
        } else if lin.red > 0.7 && lin.green > 0.45 && lin.blue < 0.15 {
            self.shard_yellow.clone()
        } else if lin.red > 0.55 && lin.green < 0.12 {
            self.shard_red.clone()
        } else if lin.green > 0.28 && lin.red < 0.12 {
            self.shard_green.clone()
        } else if lin.red > 0.5 && lin.green > 0.45 {
            self.shard_beige.clone()
        } else if lin.green > lin.red && lin.green > lin.blue {
            self.shard_olive.clone()
        } else if lin.red > 0.18 && lin.blue < 0.08 {
            self.shard_wood.clone()
        } else if lin.blue > 0.45 {
            self.shard_silver.clone()
        } else {
            self.shard_dark_metal.clone()
        }
    }
}

/// Generates randomized corporate-culture & layoff satire prop names each round!
#[derive(Resource)]
pub struct CorporateJokeRng {
    state: u64,
}

impl Default for CorporateJokeRng {
    fn default() -> Self {
        Self {
            state: 0x9E37_79B9_7F4A_7C15,
        }
    }
}

impl CorporateJokeRng {
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 16) as u32
    }

    pub fn advance_round(&mut self, extra: u64) {
        self.state = self.state.wrapping_add(extra.wrapping_mul(0x517C_C1B7_2722_0A95) | 1);
        let _ = self.next_u32();
    }

    pub fn pick<'a>(&mut self, pool: &'a [&'static str]) -> &'a str {
        let idx = (self.next_u32() as usize) % pool.len();
        pool[idx]
    }

    /// Picks a randomized corporate-satire / layoff joke label for a prop!
    pub fn label_for(&mut self, prefab: PropPrefab, custom: Option<&str>) -> String {
        // Even if a custom label exists in JSON, 65% of the time roll from the rich satire pool so every round feels fresh!
        if let Some(c) = custom
            && (self.next_u32() % 100) < 35
        {
            return c.to_string();
        }

        let chosen = match prefab {
            PropPrefab::CrtMonitor => self.pick(&[
                "ZOOM LAYOFF CALL!",
                "BADGE DEACTIVATED!",
                "BOSWARE TRACKER!",
                "9AM RIF WEBINAR!",
                "SLACK REVOKED!",
                "CALENDAR WIPEOUT!",
                "Q4 REORG DECK!",
                "360 PEER REVIEW!",
            ]),
            PropPrefab::LaptopSilver => self.pick(&[
                "HR LAYOFF SCRIPT!",
                "AI PIVOT SLIDES!",
                "EXEC BONUS SHEET!",
                "REMOTE TRACKER!",
                "LINKEDIN OPEN2WORK",
                "RTO COMPLIANCE LOG",
                "OFFSHORE ROADMAP!",
                "CEO BURNER MACBOOK",
            ]),
            PropPrefab::MechanicalKeyboard => self.pick(&[
                "REPLY-ALL KEYBOARD",
                "SLACK STATUS JIGGLER",
                "PASSIVE-AGGRESSIVE KB",
                "JIRA SPAMMER 9000",
                "MUTE-BUTTON MASHER",
            ]),
            PropPrefab::ComputerMouseAndPad => self.pick(&[
                "IDLE-TIME JIGGLER!",
                "SYNERGY MOUSEPAD!",
                "CLICK-THROUGH KPI!",
                "BOSWARE CURSOR!",
            ]),
            PropPrefab::DeskPhone => self.pick(&[
                "SURPRISE HR CALL!",
                "UNMUTED STANDUP!",
                "COLD-CALL QUOTA!",
                "FRIDAY 4:55PM CALL!",
                "SECURITY ESCORT LINE",
            ]),
            PropPrefab::PaperStackTall | PropPrefab::PaperStackMessy | PropPrefab::LoosePaperFan => {
                self.pick(&[
                    "WARN ACT NOTICES!",
                    "SURPRISE RIF LIST!",
                    "2-WEEK SEVERANCE!",
                    "NDA GAG ORDERS!",
                    "PIP PAPERWORK!",
                    "RTO MANDATE MEMO!",
                    "WE ARE FAMILY MEMO",
                    "AI REPLACEMENT PLAN",
                    "STACK-RANKING LIST!",
                    "UNPAID OVERTIME LOG",
                    "NO-PROMO PACKET!",
                    "TPS COVER SHEETS!",
                ])
            }
            PropPrefab::BinderPair | PropPrefab::PaperInTray | PropPrefab::ClipboardDesk => {
                self.pick(&[
                    "ATTRITION TARGETS!",
                    "HEADCOUNT FREEZE!",
                    "EXIT INTERVIEW LOG",
                    "LAYOFF TALKING PTS",
                    "MANDATORY FUN PLAN",
                    "CULTURE-FIT RUBRIC",
                    "SUBCONTRACTOR CUTS",
                ])
            }
            PropPrefab::CardboardBox => self.pick(&[
                "DESK CLEANOUT BOX!",
                "LAID-OFF BELONGINGS",
                "SECURITY WALKOUT BOX",
                "PACK YOUR THINGS!",
                "FORMER TEAM ARCHIVE",
                "SHREDDED PROMISES!",
            ]),
            PropPrefab::RedStapler => self.pick(&[
                "MILTON'S STAPLER!",
                "BASEMENT TRANSFER!",
                "PAYCHECK GLITCH!",
                "STOLEN DESK SWAG!",
            ]),
            PropPrefab::SteamingCoffeeMug | PropPrefab::CoffeeMakerPot | PropPrefab::RedThermos => {
                self.pick(&[
                    "NO-BONUS COFFEE!",
                    "TEARS & ESPRESSO!",
                    "FREEZE-HIRE BREW!",
                    "WORLD'S OKAYEST DEV",
                    "SURVIVED THE REORG!",
                    "SAD DESK ROAST!",
                ])
            }
            PropPrefab::WaterCooler => self.pick(&[
                "LAYOFF RUMOR MILL!",
                "GOSSIP HYDRATOR!",
                "RTO WATER COOLER!",
                "WELLNESS STIPEND!",
            ]),
            PropPrefab::MiniFridge | PropPrefab::Microwave => self.pick(&[
                "LAYOFF PIZZA FRIDGE",
                "STOLEN OAT MILK!",
                "REHEATED SALMON!",
                "PERKS BUDGET CUT!",
                "SAD DESK LUNCH!",
            ]),
            PropPrefab::CubicleWallPinned | PropPrefab::CubicleWallPoster => self.pick(&[
                "OPEN-OFFICE POD!",
                "REORG SURVIVOR POD",
                "EMPTY TEAMMATE POD",
                "HOTELING CUBICLE!",
                "QUIET-QUITTING WALL",
                "HANG IN THERE CAT!",
            ]),
            PropPrefab::FilingCabinetOlive3 | PropPrefab::FilingCabinetOlive4 => self.pick(&[
                "SECRET RIF CABINET!",
                "DENIED PROMOTIONS!",
                "GHOST-JOB RESUMES!",
                "NDA VAULT CABINET!",
                "HR INCIDENT FILES!",
            ]),
            PropPrefab::SwivelChair | PropPrefab::LeatherExecChair => self.pick(&[
                "EMPTY HEADCOUNT!",
                "MIDDLE-MGMT THRONE",
                "BACKFILLED IN 2029",
                "LUMBAR BUDGET CUT!",
                "VP GOLDEN SEAT!",
            ]),
            PropPrefab::CoworkerDesk | PropPrefab::CredenzaCabinet => self.pick(&[
                "VACANT RIF DESK!",
                "HOT-DESK ISLAND!",
                "AGILE SCRUM TABLE!",
                "DOWNSIZED STATION!",
                "EXEC SIDEBOARD!",
            ]),
            PropPrefab::PcTower => self.pick(&[
                "LEGACY MONOLITH!",
                "AI AGENT CLUSTER!",
                "KEYSTROKE LOGGER!",
                "PROD DB (NO BACKUP)",
            ]),
            PropPrefab::OfficeCopier => self.pick(&[
                "SEVERANCE PRINTER!",
                "PC LOAD LETTER!",
                "NDA COPIER JAM!",
                "PINK-SLIP DUPLEXER!",
            ]),
            PropPrefab::BookshelfManuals => self.pick(&[
                "TAX LOOPHOLE TOMES!",
                "UNION-BUSTING VOLS",
                "AGILE MANIFESTOES!",
                "SYNERGY PLAYBOOKS!",
            ]),
            PropPrefab::StockCrashTv => self.pick(&[
                "STOCK BUYBACK TV!",
                "15% RIF // RECORD EPS",
                "Q4 TOWN HALL FEED!",
                "EARNINGS PLUNGE TV!",
            ]),
            PropPrefab::SiteWhiteboard | PropPrefab::EaselChart => self.pick(&[
                "DO MORE WITH LESS!",
                "HEADCOUNT BURNDOWN",
                "SPRINT VELOCITY 0!",
                "DELAYED ROADMAP!",
                "SYNERGY QUADRANT!",
            ]),
            PropPrefab::AwardTrophy => self.pick(&[
                "CEO GOLDEN PARACHUTE",
                "RECORD LAYOFF AWARD",
                "PIZZA-PARTY TROPHY!",
                "TOP COST-CUTTER!",
            ]),
            PropPrefab::CrystalDecanter | PropPrefab::CrystalGlass | PropPrefab::BarCart => {
                self.pick(&[
                    "EXEC RETENTION BONUS",
                    "BOARDROOM BOURBON!",
                    "C-SUITE EXPENSE TAB",
                    "LAYOFF TOAST GLASS!",
                ])
            }
            PropPrefab::EspressoMachine => self.pick(&[
                "C-SUITE ESPRESSO!",
                "RTO BRIBE BOILER!",
                "OVERPRICED CORTADO!",
            ]),
            PropPrefab::GlassPartition => self.pick(&[
                "OPEN-DOOR ILLUSION!",
                "TRANSPARENCY WALL!",
                "GLASS CEILING!",
            ]),
            PropPrefab::FireExtinguisher | PropPrefab::SiteGenerator => self.pick(&[
                "PR CRISIS FOAM!",
                "DAMAGE CONTROL!",
                "RUNWAY GENERATOR!",
                "BURN-RATE ENGINE!",
            ]),
            PropPrefab::OfficeFern | PropPrefab::PottedCactus | PropPrefab::PottedSnakePlant => {
                self.pick(&[
                    "DEAD OFFICE FICUS!",
                    "LAID-OFF GUY'S PLANT",
                    "WELLNESS CACTUS!",
                    "ZERO-SUNLIGHT FERN",
                ])
            }
            PropPrefab::TrashBin => self.pick(&[
                "SUGGESTION BOX!",
                "EMPLOYEE FEEDBACK!",
                "PROMOTION REQUESTS!",
            ]),
            PropPrefab::CoatRack => self.pick(&[
                "TECH-BRO PATAGONIA!",
                "VP FLEECE VEST RACK",
                "ESCORTED OUT COATS",
            ]),
            PropPrefab::PizzaBoxStack => self.pick(&[
                "LAYOFF PIZZA PARTY!",
                "NO-RAISE PEPPERONI!",
                "1 SLICE PER SURVIVOR",
                "RECORD PROFIT PIZZA",
                "RTO BRIBE PIZZA!",
            ]),
            PropPrefab::OfficeShredder => self.pick(&[
                "SEVERANCE SHREDDER!",
                "SEC SUBPOENA EATER!",
                "PROMO FORM SHREDDER",
                "HR EVIDENCE MULCHER",
                "UNION CARD SHREDDER",
            ]),
            PropPrefab::LedVideoWall => self.pick(&[
                "15% RIF // RECORD EPS",
                "WE'RE STILL FAMILY!",
                "CEO PRE-RECORDED CRY",
                "BUYBACK LIVESTREAM!",
                "Q&A COMMENTS MUTED!",
            ]),
            PropPrefab::TeleprompterRig => self.pick(&[
                "FAKE EMPATHY SCRIPT",
                "[PAUSE FOR TEARS]!",
                "AI-WRITTEN APOLOGY!",
                "DIFFICULT NEWS RIG!",
                "BLAME MACRO HEADWIND",
            ]),
            PropPrefab::ConfettiCannon => self.pick(&[
                "BUYBACK CONFETTI!",
                "WRONG BUTTON CANNON",
                "RECORD PROFIT BLAST",
                "TONE-DEAF PYRO!",
            ]),
            PropPrefab::PaSpeakerStack => self.pick(&[
                "CORPORATE MUZAK PA!",
                "CEO MIC FEEDBACK!",
                "TOWN HALL SUBWOOFER",
                "MANDATORY CHEER AMP",
            ]),
            PropPrefab::StageMicStand => self.pick(&[
                "MUTED Q&A MIC!",
                "PLANTED SOFTBALL Q!",
                "UNPLUGGED UNION MIC",
                "EXEC LAPEL STAND!",
            ]),
            PropPrefab::RingLightRig => self.pick(&[
                "LINKEDIN TEARS HALO",
                "THOUGHT-LEADER RING",
                "APOLOGY VIDEO LIGHT",
                "FOUNDER GLOW RIG!",
            ]),
            PropPrefab::FoldingChair => self.pick(&[
                "SURVIVOR SEAT!",
                "EMPTY TEAM ROW!",
                "REORG AUDIENCE SEAT",
                "MANDATORY TOWN HALL",
                "UNCOMFORTABLE CHAIR",
            ]),
            PropPrefab::BalloonColumn => self.pick(&[
                "LAYOFF BALLOON ARCH",
                "RECORD Q4 BALLOONS!",
                "SAD MORALE PILLAR!",
                "LEFTOVER IPO ARCH!",
            ]),
            PropPrefab::VendingMachine => self.pick(&[
                "$7 BADGE-SWIPE SODA",
                "REVOKED FREE SNACKS",
                "JAMMED COIL B4!",
                "DEDUCTED FROM CHECK",
                "EXPIRED GRANOLA RIG",
            ]),
            PropPrefab::OfficeFridgeTall => self.pick(&[
                "5PM FRIDGE PURGE!",
                "STOLEN LUNCH VAULT!",
                "PASSIVE-AGGRO NOTES",
                "LABELED YOGURT TOMB",
                "BIOHAZARD TUPPERWARE",
            ]),
            PropPrefab::SheetCakeTable => self.pick(&[
                "0% COLA SHEET CAKE!",
                "SORRY U WERE RIF'D!",
                "1 CUPCAKE = BONUS!",
                "GOOD LUCK DAN CAKE!",
                "SURVIVOR FROSTING!",
            ]),
            PropPrefab::CoffeeUrn => self.pick(&[
                "LUKEWARM DECAF URN!",
                "SLUDGE MORALE VAT!",
                "BURNT 6AM BREW URN!",
                "FREE PERK (WATERED)",
            ]),
            PropPrefab::SodaCanSixPack => self.pick(&[
                "WARM LA CROIX PACK!",
                "CRUNCH-TIME REDBULL",
                "OFF-BRAND SELTZER!",
                "1 CAN PER QUARTER!",
            ]),
            PropPrefab::PlasticTrophyRack => self.pick(&[
                "5YR SERVICE LUCITE!",
                "ZERO-RAISE PLAQUES!",
                "ATTENDANCE TROPHIES",
                "TEAM PLAYER AWARD!",
            ]),
            PropPrefab::PalletBoxTower => self.pick(&[
                "PRIME CRUNCH PALLET",
                "UNSHIPPED SWAG BOXES",
                "RETURNED RTO MONITORS",
                "OVERTIME FREIGHT!",
                "2-DAY RUSH STACK!",
            ]),
            PropPrefab::ConveyorRoller => self.pick(&[
                "NON-STOP SORT LINE!",
                "NO-BREAK CONVEYOR!",
                "JAMMED PARCEL TRACK",
                "QUOTA BELT ROLLER!",
            ]),
            PropPrefab::BarcodeScannerGun => self.pick(&[
                "IDLE-SECOND SCANNER",
                "BATHROOM-BREAK GUN!",
                "PICK-RATE TRACKER!",
                "BEEP-QUOTA LASER!",
            ]),
            PropPrefab::QuotaTimerSign => self.pick(&[
                "0 DAYS NO INJURY!",
                "PICK RATE: TOO SLOW",
                "RESTROOM COUNTDOWN!",
                "ALGORITHMIC BOSS TV",
            ]),
            PropPrefab::ShrinkWrapRoll => self.pick(&[
                "RED-TAPE WRAP ROLL!",
                "PALLET STRETCH FILM",
                "GOLDEN-HANDCUFF WRAP",
            ]),
            PropPrefab::PropaneForkliftTank => self.pick(&[
                "FORKLIFT CERT VOID!",
                "OSHA VIOLATION TANK",
                "UNINSPECTED LP GAS!",
                "SHIFT-CHANGE BLAST!",
            ]),
            _ => custom.unwrap_or("CORPORATE ASSET!"),
        };

        chosen.to_string()
    }
}
