use std::collections::HashMap;
use std::io::{Error as IoError, ErrorKind};

use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetApp, AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

pub struct ScenarioDataPlugin;

impl Plugin for ScenarioDataPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ScenarioDef>()
            .init_asset_loader::<ScenarioJsonLoader>()
            .insert_resource(ScenarioRegistry::from_embedded())
            .insert_resource(CurrentScenario("corporate".to_string()));
    }
}

#[derive(Resource, Clone, Debug)]
pub struct CurrentScenario(pub String);

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioManifest {
    pub scenarios: Vec<ScenarioManifestEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioManifestEntry {
    pub id: String,
    pub title: String,
    pub short_label: String,
    pub file: String,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioDef {
    pub id: String,
    pub title: String,
    pub short_label: String,
    pub theme: RoomThemeDef,
    pub lighting: LightingDef,
    pub desk: DeskDef,
    pub props: Vec<PropPlacement>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoomThemeDef {
    pub floor_texture: String,
    pub wall_texture: String,
    pub ceiling_texture: String,
    pub trim_texture: String,
    pub window_style: String,
    #[serde(default)]
    pub has_left_blind_window: bool,
    #[serde(default)]
    pub has_pillar_clock: bool,
    #[serde(default = "default_sunbeam")]
    pub sunbeam_color: [f32; 4],
}

fn default_sunbeam() -> [f32; 4] {
    [1.0, 0.80, 0.40, 0.14]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LightingDef {
    pub clear_color: [f32; 3],
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
    pub sun_color: [f32; 3],
    pub sun_illuminance: f32,
    pub sun_pos: [f32; 3],
    pub sun_look: [f32; 3],
    pub fill_color: [f32; 3],
    pub fill_intensity: f32,
    pub fill_pos: [f32; 3],
    pub accent_color: [f32; 3],
    pub accent_intensity: f32,
    pub accent_pos: [f32; 3],
    pub fog_color: [f32; 4],
    pub fog_start: f32,
    pub fog_end: f32,
    pub shadow_tint: [f32; 3],
    pub highlight_tint: [f32; 3],
    pub bloom_strength: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeskDef {
    pub archetype: String,
    pub pos: [f32; 3],
    pub mass: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PropPlacement {
    pub prefab: PropPrefab,
    pub pos: [f32; 3],
    #[serde(default)]
    pub rot_y: f32,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub points: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropPrefab {
    CrtMonitor,
    MechanicalKeyboard,
    ComputerMouseAndPad,
    DeskPhone,
    DeskLamp,
    PencilCup,
    PictureFrameDesk,
    RedStapler,
    SteamingCoffeeMug,
    PaperStackTall,
    PaperStackMessy,
    LoosePaperFan,
    CardboardBox,
    CoatRack,
    CoffeeMakerPot,
    Microwave,
    PottedCactus,
    PottedSnakePlant,
    CubicleWallPinned,
    CubicleWallPoster,
    FilingCabinetOlive3,
    FilingCabinetOlive4,
    BinderPair,
    PaperInTray,
    WaterCooler,
    TrashBin,
    SwivelChair,
    LeatherExecChair,
    CoworkerDesk,
    PcTower,
    OfficeCopier,
    BookshelfManuals,
    OfficeFern,
    FluorescentFixture,
    HardHat,
    ClipboardDesk,
    TapeMeasure,
    RolledBlueprints,
    RedThermos,
    WalkieTalkie,
    SiteWhiteboard,
    MiniFridge,
    TrafficCone,
    RedToolbox,
    Sawhorse,
    CinderBlockStack,
    SiteGenerator,
    ShelvingRack,
    PaintCanBlue,
    PaintCanRed,
    PaintCanYellow,
    FireExtinguisher,
    StepLadder,
    LaptopSilver,
    CrystalDecanter,
    CrystalGlass,
    BarCart,
    AwardTrophy,
    CredenzaCabinet,
    EaselChart,
    PizzaBoxStack,
    OfficeShredder,
    StockCrashTv,
    GlassPartition,
    EspressoMachine,
    // Scene 4: All-Hands Town Hall Stage
    LedVideoWall,
    TeleprompterRig,
    ConfettiCannon,
    PaSpeakerStack,
    StageMicStand,
    RingLightRig,
    FoldingChair,
    BalloonColumn,
    // Scene 5: Sad Corporate Breakroom
    VendingMachine,
    OfficeFridgeTall,
    SheetCakeTable,
    CoffeeUrn,
    SodaCanSixPack,
    PlasticTrophyRack,
    // Scene 6: Fulfillment Warehouse Cage
    PalletBoxTower,
    ConveyorRoller,
    BarcodeScannerGun,
    QuotaTimerSign,
    ShrinkWrapRoll,
    PropaneForkliftTank,
}

#[derive(Default, TypePath)]
pub struct ScenarioJsonLoader;

impl AssetLoader for ScenarioJsonLoader {
    type Asset = ScenarioDef;
    type Settings = ();
    type Error = IoError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        serde_json::from_slice::<ScenarioDef>(&bytes)
            .map_err(|e| IoError::new(ErrorKind::InvalidData, e))
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// Holds parsed scenarios (both instant embedded defaults and dynamically loaded `/assets/scenarios/*.json`).
#[derive(Resource)]
pub struct ScenarioRegistry {
    pub order: Vec<String>,
    pub defs: HashMap<String, ScenarioDef>,
    pub pending_dynamic: HashMap<String, Handle<ScenarioDef>>,
}

impl ScenarioRegistry {
    pub fn from_embedded() -> Self {
        let manifest_raw = include_str!("../../../web/public/assets/scenarios/manifest.json");
        let raw_files = [
            include_str!("../../../web/public/assets/scenarios/corporate.json"),
            include_str!("../../../web/public/assets/scenarios/building_site.json"),
            include_str!("../../../web/public/assets/scenarios/boardroom.json"),
            include_str!("../../../web/public/assets/scenarios/all_hands.json"),
            include_str!("../../../web/public/assets/scenarios/breakroom.json"),
            include_str!("../../../web/public/assets/scenarios/warehouse.json"),
        ];

        let mut defs = HashMap::new();
        for raw in raw_files {
            if let Ok(def) = serde_json::from_str::<ScenarioDef>(raw) {
                defs.insert(def.id.clone(), def);
            }
        }

        let mut order: Vec<String> = serde_json::from_str::<ScenarioManifest>(manifest_raw)
            .map(|m| {
                m.scenarios
                    .into_iter()
                    .filter(|e| !e.file.is_empty() && !e.title.is_empty() && !e.short_label.is_empty())
                    .map(|e| e.id)
                    .collect()
            })
            .unwrap_or_default();
        if order.is_empty() {
            order = vec![
                "corporate".to_string(),
                "building_site".to_string(),
                "boardroom".to_string(),
            ];
        }

        Self {
            order,
            defs,
            pending_dynamic: HashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<&ScenarioDef> {
        self.defs.get(id)
    }

    pub fn title_for(&self, id: &str) -> &str {
        self.defs
            .get(id)
            .map(|d| d.title.as_str())
            .unwrap_or("OFFICE CHAOS")
    }

    pub fn next_after(&self, current_id: &str) -> String {
        if self.order.is_empty() {
            return "corporate".to_string();
        }
        let idx = self
            .order
            .iter()
            .position(|k| k == current_id)
            .unwrap_or(0);
        self.order[(idx + 1) % self.order.len()].clone()
    }

    pub fn by_index(&self, zero_based: usize) -> Option<&str> {
        self.order.get(zero_based).map(String::as_str)
    }
}
