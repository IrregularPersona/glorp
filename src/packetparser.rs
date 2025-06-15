use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct ParsedPacket {
    pub opcode: String,
    pub meaning: String,
    pub parsed: PacketData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PacketData {
    Spawn(SpawnData),
    Move(MoveData),
    UpdatePlayers(UpdatePlayersData),
    LobbyMove(LobbyMoveData),
    Hit(HitData),
    Chat(ChatData),
    GameState(GameStateData),
    PlayerSync(PlayerSyncData),
    PlayerInit(PlayerInitData),
    Generic(GenericData),
}

#[derive(Debug, Serialize)]
pub struct SpawnData {
    pub r#type: String,
    pub name: String,
    pub hp: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rot_y: f64,
    pub pitch: f64,
    pub z_vel: f64,
    pub team: i32,
}

#[derive(Debug, Serialize)]
pub struct MoveData {
    pub r#type: String,
    pub pos_map: Value,
    pub path: Value,
}

// --- Start k packets
#[derive(Debug, Serialize)]
pub struct ViewAngles {
    pub yaw: i32,                               // Horizontal look direction
    pub pitch: i32,                             // Vertical look direction
}

#[derive(Debug, Serialize)]
pub struct PlayerStateFlags {
    pub on_ground: bool,                        // true = grounded, false = airborne
    pub is_crouching: bool,                     // true = crouching
    pub using_secondary: bool,                  // true = secondary weapon
    pub hip_firing: bool,                       // true = hipfire, false = ADS (aim-down-sight)
    pub flag_11: bool,                          // Unknown, needs context
}

#[derive(Debug, Serialize)]
pub struct UpdatePlayersData {
    pub player_id: i32,
    pub first_focused_position: Position,       // Whoever gets the latest kills gets tracked on the top
    pub view_angles: ViewAngles,                // first focused player yaw/pitch
    pub momentum_a: f64,                        // speed or momentum stuff (?)
    pub momentum_b: f64,                        // speed or momentum stuff (?)
    pub focused_player_ping: i32,               // "Position"s player ping ???? WHY 😭😭 
    pub team_id: i32,                           // Need more context (placeholder name)
    pub state_flags: PlayerStateFlags,          // Specific State Flags
    pub second_focused_position: Position,      // x, y, z position on map
    pub second_focused_view_angles: ViewAngles, // first focused player yaw/pitch
}
// --- End k packets

// --- START h PACKETS
#[derive(Debug, Deserialize)]
pub struct GetHealthData {
    pub opcode: String,
    pub curr_health: i32,
    pub pos_shooter_1: i32,             // No clue between X or Z
    pub pos_shooter_2: i32,             // No clue between X or Z
}
// --- END h PACKETS

// --- START 6 PACKETS
#[derive(Debug, Deserialize)]
pub struct GetKillData {
    pub opcode: String,
    pub kill_score: i32,
}

#[derive(Debug, Serialize)]
pub struct LobbyMoveData {
    pub r#type: String,
    pub player_id: String,
    pub code: String,
    pub session_id: String,
    pub state: i32,
}

#[derive(Debug, Serialize)]
pub struct HitData {
    pub r#type: String,
    pub attacker_id: i32,
    pub victim_id: i32,
    pub damage: f64,
    pub effects: Value,
    pub hit_info: HitInfo,
    pub meta: Value,
}

#[derive(Debug, Serialize)]
pub struct HitInfo {
    pub distance: f64,
    pub is_headshot: bool,
    pub is_wallbang: bool,
    pub weapon_id: i32,
}

#[derive(Debug, Serialize)]
pub struct ChatData {
    pub r#type: String,
    pub category: i32,
    pub message: String,
    pub player: String,
    pub priority: i32,
    pub meta: Value,
    pub meaning: String,
}

#[derive(Debug, Serialize)]
pub struct GameStateData {
    pub r#type: String,
    pub state_code: i32,
    pub meaning: String,
}

#[derive(Debug, Serialize)]
pub struct PlayerSyncData {
    pub r#type: String,
    pub id: i32,
    pub team: i32,
    pub position: Position,
    pub velocity: Position,
    pub rotation: f64,
    pub flags: PlayerFlags,
}

#[derive(Debug, Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Serialize)]
pub struct PlayerFlags {
    pub flag1: Value, pub flag2: Value, pub flag3: Value, pub flag4: Value,
    pub flag5: Value, pub flag6: Value, pub flag7: Value, pub flag8: Value,
    pub flag9: Value, pub flag10: Value, pub flag11: Value, pub flag12: Value,
    pub flag13: Value, pub flag14: Value, pub flag15: Value, pub flag16: Value,
}

#[derive(Debug, Serialize)]
pub struct PlayerInitData {
    pub r#type: String,
    pub uid: i32,
    pub class_id: i32,
    pub score: i32,
    pub position: Position2D,
    pub username: String,
    pub alive: bool,
    pub health: f64,
    pub max_health: f64,
    pub clan: String,
    pub angle: f64,
    pub game_mode: i32,
    pub weapon_id: i32,
    pub raw: Vec<Value>,
    pub broadcast_flag: Value,
}

#[derive(Debug, Serialize)]
pub struct Position2D {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize)]
pub struct GenericData {
    pub r#type: String,
    pub name: String,
    pub raw: Vec<Value>,
}

pub struct PacketParser {
    opcode_names: HashMap<String, String>,
}

impl PacketParser {
    pub fn new() -> Self {
        let mut opcode_names = HashMap::new();
        
        opcode_names.insert("_0".to_string(), "RPC_SUCCESS".to_string());
        opcode_names.insert("_1".to_string(), "RPC_ERROR".to_string());
        opcode_names.insert("cpt".to_string(), "cpt".to_string());
        opcode_names.insert("cptR".to_string(), "cptR".to_string());
        opcode_names.insert("fp".to_string(), "FORCE_PING".to_string());
        opcode_names.insert("n".to_string(), "NUKE_UPDATE".to_string());
        opcode_names.insert("bbm".to_string(), "BOMBARDMENT_UPDATE".to_string());
        opcode_names.insert("0".to_string(), "ADD_PLAYERS".to_string());
        opcode_names.insert("00".to_string(), "UPDATE_NAMES".to_string());
        opcode_names.insert("k".to_string(), "UPDATE_PLAYERS".to_string());
        opcode_names.insert("code".to_string(), "ENTER_CODE".to_string());
        opcode_names.insert("codeT".to_string(), "UPDATE_CODE_TIMER".to_string());
        opcode_names.insert("codeP".to_string(), "CODE_PASSED".to_string());
        opcode_names.insert("init".to_string(), "INIT_GAME".to_string());
        opcode_names.insert("io-init".to_string(), "IO_INIT".to_string());
        opcode_names.insert("inst-id".to_string(), "GAME_INSTANCE_ID".to_string());
        opcode_names.insert("load".to_string(), "INIT_LOAD".to_string());
        opcode_names.insert("vr".to_string(), "REDEEM_VOUCHER_RESPONSE".to_string());
        opcode_names.insert("kml".to_string(), "ADD_MAIL".to_string());
        opcode_names.insert("cust".to_string(), "CUSTOM_RESPONSE".to_string());
        opcode_names.insert("ready".to_string(), "READY".to_string());
        opcode_names.insert("cln".to_string(), "CLAN_RESPONSE".to_string());
        opcode_names.insert("start".to_string(), "START_GAME".to_string());
        opcode_names.insert("nwT".to_string(), "NEW_TOKEN".to_string());
        opcode_names.insert("pur".to_string(), "PURCHASE_RESPONSE".to_string());
        opcode_names.insert("ppr".to_string(), "PURCHASE_PRODUCT_RESPONSE".to_string());
        opcode_names.insert("purprm".to_string(), "PURCHASE_PREMIUM_RESPONSE".to_string());
        opcode_names.insert("purb".to_string(), "PURCHASE_BUNDLE_RESPONSE".to_string());
        opcode_names.insert("uprm".to_string(), "UPDATE_PREMIUM_AMOUNT".to_string());
        opcode_names.insert("purbp".to_string(), "PURCHASE_RESPONSE_2".to_string());
        opcode_names.insert("purbpl".to_string(), "PURCHASE_LEVEL_RESPONSE".to_string());
        opcode_names.insert("bpclmr".to_string(), "CLAIM_ITEM_RESPONSE".to_string());
        opcode_names.insert("gmsg".to_string(), "UPDATE_GAME_MESSAGE".to_string());
        opcode_names.insert("lkt".to_string(), "UPDATE_LOCKOUT_TIM".to_string());
        opcode_names.insert("inat".to_string(), "PLAYER_INTERACTIONS".to_string());
        opcode_names.insert("inatA".to_string(), "ALL_PLAYER_INTERACTIONS".to_string());
        opcode_names.insert("sb".to_string(), "SHOW_SPEECH_BUBBLE".to_string());
        opcode_names.insert("clnL".to_string(), "CLAN_LEAVE_RESPONSE".to_string());
        opcode_names.insert("upR".to_string(), "UPLOAD_RESPONSE".to_string());
        opcode_names.insert("upPR".to_string(), "UPLOAD_PROFILE_RESPONSE".to_string());
        opcode_names.insert("lock".to_string(), "LOCK_PLAYER".to_string());
        opcode_names.insert("end".to_string(), "END_GAME".to_string());
        opcode_names.insert("spin".to_string(), "SPIN_RESPONSE".to_string());
        opcode_names.insert("spinH".to_string(), "SAVE_SPIN_HISTORY".to_string());
        opcode_names.insert("spinW".to_string(), "SPIN_WHEEL_RESPONSE".to_string());
        opcode_names.insert("skin".to_string(), "SKIN_REWARD".to_string());
        opcode_names.insert("skinF".to_string(), "SKIN_FOUND_MESSAGE".to_string());
        opcode_names.insert("spinKR".to_string(), "SPIN_KR".to_string());
        opcode_names.insert("ts".to_string(), "TEAM_SCORE".to_string());
        opcode_names.insert("t".to_string(), "UPDATE_TIMER".to_string());
        opcode_names.insert("pErr".to_string(), "PURCHASE_ERROR".to_string());
        opcode_names.insert("error".to_string(), "ERROR".to_string());
        opcode_names.insert("dc".to_string(), "DISCONNECT".to_string());
        opcode_names.insert("unb".to_string(), "UNBOX_MESSAGE".to_string());
        opcode_names.insert("unbS".to_string(), "UNBOX_MESSSAGE_S".to_string());
        opcode_names.insert("zn".to_string(), "UPDATE_ZONE".to_string());
        opcode_names.insert("uf".to_string(), "UPDATE_FUNDS".to_string());
        opcode_names.insert("chlR".to_string(), "UPDATE_CHALLENGE_REWARDS".to_string());
        opcode_names.insert("l".to_string(), "SYNC_PLAYER".to_string());
        opcode_names.insert("f".to_string(), "SYNC_FAILED".to_string());
        opcode_names.insert("ufi".to_string(), "UPDATE_FLAG_INFO".to_string());
        opcode_names.insert("debug".to_string(), "SET_DEBUG".to_string());
        opcode_names.insert("exf".to_string(), "SET_EXPERIMENTAL".to_string());
        opcode_names.insert("noc".to_string(), "SET_NOCLIP".to_string());
        opcode_names.insert("frz".to_string(), "SET_FROZEN".to_string());
        opcode_names.insert("spb".to_string(), "SET_SPEED_BOOST".to_string());
        opcode_names.insert("unl".to_string(), "SET_UNLIMITED".to_string());
        opcode_names.insert("sic".to_string(), "SET_SHARING_IS_CARING".to_string());
        opcode_names.insert("s".to_string(), "PLAYER_SOUND".to_string());
        opcode_names.insert("c".to_string(), "PLAYER_ANIM".to_string());
        opcode_names.insert("7".to_string(), "UPDATE_LEADERS".to_string());
        opcode_names.insert("sp".to_string(), "ADD_SPRAY".to_string());
        opcode_names.insert("ch".to_string(), "ADD_CHAT".to_string());
        opcode_names.insert("chi".to_string(), "ADD_CHATI18N".to_string());
        opcode_names.insert("tch".to_string(), "ADD_TRADE_CHAT".to_string());
        opcode_names.insert("9".to_string(), "SHOW_TRACER".to_string());
        opcode_names.insert("10".to_string(), "GET_ASSIST".to_string());
        opcode_names.insert("h".to_string(), "CHANGE_HEALTH".to_string());
        opcode_names.insert("so".to_string(), "PLAY_SOUND".to_string());
        opcode_names.insert("sso".to_string(), "STOP_SOUND".to_string());
        opcode_names.insert("6".to_string(), "GET_KILL".to_string());
        opcode_names.insert("p".to_string(), "INTER_PROG".to_string());
        opcode_names.insert("mp".to_string(), "INTER_M_PROG".to_string());
        opcode_names.insert("lv".to_string(), "UPDATE_LIVES".to_string());
        opcode_names.insert("ua".to_string(), "UPDATE_ACCOUNT".to_string());
        opcode_names.insert("ex".to_string(), "EXPLOSION".to_string());
        opcode_names.insert("ac".to_string(), "ACTION_FEED".to_string());
        opcode_names.insert("vc".to_string(), "VOICE_CHAT".to_string());
        opcode_names.insert("a".to_string(), "ACCOUNT_RESPONSE".to_string());
        opcode_names.insert("4".to_string(), "DO_DAMAGE".to_string());
        opcode_names.insert("5".to_string(), "GET_SCORE".to_string());
        opcode_names.insert("ufp".to_string(), "UPDATE_FLAG_PING".to_string());
        opcode_names.insert("fh".to_string(), "SET_HIDDEN".to_string());
        opcode_names.insert("bnk".to_string(), "UPDATE_BANK".to_string());
        opcode_names.insert("uamo".to_string(), "UPDATE_AMMO".to_string());
        opcode_names.insert("upp".to_string(), "NEW_PASS_RESPONSE".to_string());
        opcode_names.insert("ro".to_string(), "RESPAWN_OBJ".to_string());
        opcode_names.insert("st".to_string(), "KILL_STREAK".to_string());
        opcode_names.insert("pk".to_string(), "EFFECT".to_string());
        opcode_names.insert("upk".to_string(), "UPDATE_ZOMBIE_PERKS".to_string());
        opcode_names.insert("2".to_string(), "REMOVE_PLAYER".to_string());
        opcode_names.insert("dbgp".to_string(), "ADD_DEBUG_POING".to_string());
        opcode_names.insert("recps".to_string(), "CRAFT_DATA_RESPONSE".to_string());
        opcode_names.insert("crftr".to_string(), "CRAFT_RESPONSE".to_string());
        opcode_names.insert("sd".to_string(), "CUSTOM_NETWORK_MESSAGE".to_string());
        opcode_names.insert("kpdf".to_string(), "KPD_RESET".to_string());
        opcode_names.insert("kpd".to_string(), "KPD_RESPONSE".to_string());
        opcode_names.insert("clog".to_string(), "CONSOLE_LOG".to_string());
        opcode_names.insert("wstk".to_string(), "UPDATE_WEAPON_STREAK".to_string());
        opcode_names.insert("krp".to_string(), "KR_PACKAGE".to_string());
        opcode_names.insert("ufl".to_string(), "UPDATE_FLAG".to_string());
        opcode_names.insert("kst".to_string(), "KILL_STREAK_M".to_string());
        opcode_names.insert("am".to_string(), "ADD_MEDAL".to_string());
        opcode_names.insert("gt".to_string(), "LOG_TIME".to_string());
        opcode_names.insert("gte".to_string(), "UPDATE_GATE".to_string());
        opcode_names.insert("mybx".to_string(), "UPDATE_MYSTERY_BOX".to_string());
        opcode_names.insert("smybx".to_string(), "SWITCH_MYSTERY_BOX".to_string());
        opcode_names.insert("krm".to_string(), "UPDATE_KRUM_MACHINE".to_string());
        opcode_names.insert("pwup".to_string(), "UPDATE_POWER_UP".to_string());
        opcode_names.insert("lm".to_string(), "LOCK_MOVE".to_string());
        opcode_names.insert("rml".to_string(), "OPEN_MAIL".to_string());
        opcode_names.insert("gmc".to_string(), "UPDATE_MAIL".to_string());
        opcode_names.insert("chp".to_string(), "CHECK_POINT_SET".to_string());
        opcode_names.insert("chgp".to_string(), "CHANGE_POSITION".to_string());
        opcode_names.insert("ulm".to_string(), "UNLOCK_MOVE".to_string());
        opcode_names.insert("is".to_string(), "TRIGGER_IMG_SOUND".to_string());
        opcode_names.insert("pi".to_string(), "SOCKET_PING".to_string());
        opcode_names.insert("mv".to_string(), "UPD_MATCH_VOTE".to_string());
        opcode_names.insert("frs".to_string(), "ENTER_GAME".to_string());
        opcode_names.insert("chg".to_string(), "UPDATE_CHALLENGES".to_string());
        opcode_names.insert("uchp".to_string(), "UPDATE_CHALLENGES_PROGRESS".to_string());
        opcode_names.insert("vk".to_string(), "VOTE_KICK".to_string());
        opcode_names.insert("vf".to_string(), "VOTE_FORFEIT".to_string());
        opcode_names.insert("pir".to_string(), "SOCKET_PING_RESULT".to_string());
        opcode_names.insert("scr".to_string(), "JUMP_SCARE".to_string());
        opcode_names.insert("up".to_string(), "UPDATE_PROP".to_string());
        opcode_names.insert("prpR".to_string(), "UPDATE_PROP_ROT".to_string());
        opcode_names.insert("pr".to_string(), "SHOW_PROJECTILE".to_string());
        opcode_names.insert("bmb".to_string(), "SHOW_BOMBARDMENT".to_string());
        opcode_names.insert("chgC".to_string(), "CHALLENGE_COMPLETED".to_string());
        opcode_names.insert("flgr".to_string(), "COUNTRY_FLAG_UPDATE".to_string());
        opcode_names.insert("bdgr".to_string(), "BADGE_UPDATE".to_string());
        opcode_names.insert("cbdgr".to_string(), "BADGE_CLAIM_RESPONSE".to_string());
        opcode_names.insert("nbdgr".to_string(), "BADGE_CLAIM_MULTI_RES".to_string());
        opcode_names.insert("hdsr".to_string(), "HIDE_STATUS".to_string());
        opcode_names.insert("tm".to_string(), "UPDATE_TEAM".to_string());
        opcode_names.insert("pre".to_string(), "END_PROJECTILE".to_string());
        opcode_names.insert("wep".to_string(), "SET_WEAPON".to_string());
        opcode_names.insert("do".to_string(), "DESTROY_GAMEOBJECT".to_string());
        opcode_names.insert("mo".to_string(), "TELEPORT_TO_NODE".to_string());
        opcode_names.insert("mf".to_string(), "TELEPORT_FROM_NODE".to_string());
        opcode_names.insert("obj".to_string(), "SET_OBJECTIVE".to_string());
        opcode_names.insert("uobj".to_string(), "UPDATE_OBJECTIVE".to_string());
        opcode_names.insert("prmR".to_string(), "PREMIUM_RESPONSE".to_string());
        opcode_names.insert("nmeR".to_string(), "ALIAS_RESPONSE".to_string());
        opcode_names.insert("trd".to_string(), "TRADE_RESPONSE".to_string());
        opcode_names.insert("pRes".to_string(), "SHOW_POPUP".to_string());
        opcode_names.insert("lmsg".to_string(), "LOADING_POPUP".to_string());
        opcode_names.insert("ann".to_string(), "ANNOUNCEMENT".to_string());
        opcode_names.insert("utrd".to_string(), "UPDATE_TRADE_RESPONSE".to_string());
        opcode_names.insert("atrd".to_string(), "TRADE_A_RESPONSE".to_string());
        opcode_names.insert("twitchDropsResponse".to_string(), "TWITCH_DROPS_RESPONSE".to_string());
        opcode_names.insert("twitchRemoveResponse".to_string(), "TWITCH_REMOVE_RESPONSE".to_string());
        opcode_names.insert("twitchVerifyResponse".to_string(), "TWITCH_VERIFY_RESPONSE".to_string());
        opcode_names.insert("nftR".to_string(), "NFT_RESPONSE".to_string());
        opcode_names.insert("pdR".to_string(), "PROMOS_RESPONSE".to_string());
        opcode_names.insert("dsrvl".to_string(), "DEDI_BUY_RESPONSE".to_string());
        opcode_names.insert("gfrnd".to_string(), "FRIENDS_LIST_RESPONSE".to_string());
        opcode_names.insert("vm".to_string(), "VOTE_MAP".to_string());
        opcode_names.insert("loadS".to_string(), "UPDATE_SETTINGS".to_string());
        opcode_names.insert("crsp".to_string(), "CREATE_SPAWNABLE".to_string());
        opcode_names.insert("dsp".to_string(), "DISPOSE_SPAWNABLE".to_string());
        opcode_names.insert("bo".to_string(), "BUILD_OBJECT".to_string());
        opcode_names.insert("aai".to_string(), "C_SPAWN".to_string());
        opcode_names.insert("rai".to_string(), "REMOVE_BY_SID".to_string());
        opcode_names.insert("ai".to_string(), "SYNC_ALL_AI".to_string());
        opcode_names.insert("ad".to_string(), "SYNC_AIRDROP".to_string());
        opcode_names.insert("rad".to_string(), "REMOVE_AIRDROP_BY_SID".to_string());
        opcode_names.insert("ana".to_string(), "SERV_ANIM".to_string());
        opcode_names.insert("spk".to_string(), "SET_PLAYER_KEY".to_string());
        opcode_names.insert("rfl".to_string(), "REFILL_KNIFE".to_string());
        opcode_names.insert("cv".to_string(), "CUSTOM_VAL".to_string());
        opcode_names.insert("cnfm".to_string(), "CONFIRM_INTERACTION".to_string());
        opcode_names.insert("edMp".to_string(), "EDIT_MAP".to_string());
        opcode_names.insert("xst".to_string(), "LOAD_XSOLLA".to_string());
        opcode_names.insert("rg".to_string(), "REGEN_PLAYER".to_string());
        opcode_names.insert("ulb".to_string(), "UPDATE_LEADER_BOARD".to_string());
        opcode_names.insert("rds".to_string(), "UPDATE_ROUNDS_DIS".to_string());
        opcode_names.insert("gv".to_string(), "UPDATE_GLOBAL_VALUE".to_string());
        opcode_names.insert("cd".to_string(), "SYNC_P_CON_DATA".to_string());
        opcode_names.insert("cda".to_string(), "SYNC_P_CON_OBJ".to_string());
        opcode_names.insert("tgui".to_string(), "TOGGLE_ELEMENT".to_string());
        opcode_names.insert("fe".to_string(), "FAILED_TO_ENDER".to_string());
        opcode_names.insert("tx".to_string(), "TEXT_POPUP".to_string());
        opcode_names.insert("upz".to_string(), "UPDATE_ZOMBIE_ROUND".to_string());
        opcode_names.insert("uid".to_string(), "CON_UID".to_string());
        opcode_names.insert("hlpr".to_string(), "ADD_HELPER_C".to_string());
        opcode_names.insert("dch".to_string(), "DEBUG_COMPLEX_HITBOX".to_string());
        opcode_names.insert("hbdb".to_string(), "ADD_HITBOX_HELPER".to_string());
        opcode_names.insert("shk".to_string(), "SHAKE".to_string());
        opcode_names.insert("ksL".to_string(), "SCRIPT_LOG".to_string());
        opcode_names.insert("cct".to_string(), "CAPTURE_CONTROL".to_string());
        opcode_names.insert("uct".to_string(), "UPDATE_DOM_ZONE".to_string());
        opcode_names.insert("chrg".to_string(), "SET_PLAYER_CHARGED".to_string());
        opcode_names.insert("uai".to_string(), "UPDATE_ASSET_ID".to_string());
        opcode_names.insert("uh".to_string(), "UPDATE_HOST".to_string());
        opcode_names.insert("camb".to_string(), "CHANGE_AMBIENT".to_string());
        opcode_names.insert("jnk".to_string(), "FOUND_JUNK".to_string());
        opcode_names.insert("gbdr".to_string(), "AVAILABLE_BUNDLE_DATA".to_string());
        opcode_names.insert("abd".to_string(), "ALL_BUNDLE_DATA".to_string());
        opcode_names.insert("vad".to_string(), "FORCE_VIDEO_AD".to_string());
        opcode_names.insert("vmm".to_string(), "OPEN_METAMASK_AUTH".to_string());
        opcode_names.insert("vmms".to_string(), "METAMASK_LINK_RESPONSE".to_string());
        opcode_names.insert("rmm".to_string(), "METAMASK_UNLINK_RESPONSE".to_string());
        opcode_names.insert("krt".to_string(), "UPDATE_KR_TAGS".to_string());
        opcode_names.insert("bcfg".to_string(), "BOTS_POPUP".to_string());
        opcode_names.insert("jg".to_string(), "SET_JUGGERNAUT".to_string());
        opcode_names.insert("busy".to_string(), "SET_BUSY".to_string());
        opcode_names.insert("asnd".to_string(), "ADD_PLAYER_SOUND".to_string());
        opcode_names.insert("rsnd".to_string(), "REMOVE_PLAYER_SOUND".to_string());
        opcode_names.insert("cntry".to_string(), "UPDATE_COUNTRY".to_string());
        opcode_names.insert("bh".to_string(), "UPDATE_HEAD_SIZE".to_string());
        opcode_names.insert("bpdl".to_string(), "DATA_LOADED".to_string());
        opcode_names.insert("rrqdxp".to_string(), "RECIEVE_DOUBLE_XP".to_string());
        opcode_names.insert("bppd".to_string(), "SYNC_BP_PROG".to_string());
        opcode_names.insert("svybRes".to_string(), "SURVEY_BUNDLE_RESPONSE".to_string());
        opcode_names.insert("gameAnnouncement".to_string(), "GAME_ACCOUNCEMENT".to_string());
        opcode_names.insert("lo".to_string(), "SYNC_NEW_OBJECTS".to_string());
        opcode_names.insert("slo".to_string(), "SYNC_LIVE_OBJECTS".to_string());
        opcode_names.insert("prc".to_string(), "PLAYER_RECONNECTED".to_string());
        opcode_names.insert("gsc".to_string(), "GAME_STATE_CHANGED".to_string());
        opcode_names.insert("debug_rc".to_string(), "DEBUG_RAYCAST".to_string());
        opcode_names.insert("mpos".to_string(), "MARK_POSITION".to_string());
        opcode_names.insert("prr".to_string(), "POPUP_REGISTER_REWARDS".to_string());
        
        Self { opcode_names }
    }

    pub fn get_opcode_name(&self, code: &str) -> String {
        self.opcode_names.get(code)
            .cloned()
            .unwrap_or_else(|| format!("UNKNOWN ({})", code))
    }

    pub fn parse_packet(&self, packet: &[Value]) -> Result<ParsedPacket, String> {
        if packet.is_empty() {
            return Err("Empty packet".to_string());
        }

        let opcode = packet[0].as_str().ok_or("Invalid opcode")?;
        let meaning = self.get_opcode_name(opcode);

        let parsed = match opcode {
            "sp" => self.parse_spawn(packet)?,
            "mv" => self.parse_move(packet)?,
            "k" => self.parse_update_players(packet)?,
            "q" => self.parse_lobby_move(packet)?,
            "3" => self.parse_hit(packet)?,
            "gsc" => self.parse_game_state(packet)?,
            "chi" => self.parse_chat(packet)?,
            "l" => self.parse_player_sync(packet)?,
            "0" => self.parse_player_init(packet)?,
            _ => PacketData::Generic(GenericData {
                r#type: opcode.to_string(),
                name: meaning.clone(),
                raw: packet.to_vec(),
            }),
        };

        Ok(ParsedPacket {
            opcode: opcode.to_string(),
            meaning,
            parsed,
        })
    }

    fn parse_spawn(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 10 {
            return Err("Invalid spawn packet length".to_string());
        }

        Ok(PacketData::Spawn(SpawnData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            name: packet[1].as_str().unwrap_or("").to_string(),
            hp: packet[2].as_f64().unwrap_or(0.0),
            x: packet[3].as_f64().unwrap_or(0.0),
            y: packet[4].as_f64().unwrap_or(0.0),
            z: packet[5].as_f64().unwrap_or(0.0),
            rot_y: packet[6].as_f64().unwrap_or(0.0),
            pitch: packet[7].as_f64().unwrap_or(0.0),
            z_vel: packet[8].as_f64().unwrap_or(0.0),
            team: packet[9].as_i64().unwrap_or(0) as i32,
        }))
    }

    fn parse_move(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 3 {
            return Err("Invalid move packet length".to_string());
        }

        Ok(PacketData::Move(MoveData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            pos_map: packet[1].clone(),
            path: packet[2].clone(),
        }))
    }

    fn parse_update_players(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 2 {
            return Err("Invalid update_players packet length".to_string());
        }
        let data = packet[1].as_array().ok_or("Invalid update_players data format")?;
        if data.len() < 13 {
            return Err(format!("Insufficient update_players data length: expected 13, got {}", data.len()));
        }

        let to_bool = |v: &Value| v.as_i64().unwrap_or(0) == 1;

        Ok(PacketData::UpdatePlayers(UpdatePlayersData {
            player_id: data[0].as_i64().unwrap_or(0) as i32,
            first_focused_position: Position {
                x: data[1].as_f64().unwrap_or(0.0),
                y: data[2].as_f64().unwrap_or(0.0),
                z: data[3].as_f64().unwrap_or(0.0),
            },
            view_angles: ViewAngles {
                yaw: data[4].as_i64().unwrap_or(0) as i32,
                pitch: data[5].as_i64().unwrap_or(0) as i32,
            },
            momentum_a: data[6].as_f64().unwrap_or(0.0),
            momentum_b: data[19].as_f64().unwrap_or(0.0),
            focused_player_ping: data[12].as_i64().unwrap_or(0) as i32,
            team_id: data[13].as_i64().unwrap_or(0) as i32,

            state_flags: PlayerStateFlags {
                on_ground: to_bool(&data[7]), 
                is_crouching: to_bool(&data[8]), 
                using_secondary: to_bool(&data[9]), 
                hip_firing: to_bool(&data[10]), 
                flag_11: to_bool(&data[11])
            },

            second_focused_position: Position {
                x: data[14].as_f64().unwrap_or(0.0),
                y: data[15].as_f64().unwrap_or(0.0),
                z: data[16].as_f64().unwrap_or(0.0),
            },
            
            second_focused_view_angles: ViewAngles { 
                yaw: data[17].as_i64().unwrap_or(0) as i32, 
                pitch: data[18].as_i64().unwrap_or(0) as i32 
            }
        }))
    }


    fn parse_lobby_move(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 5 {
            return Err("Invalid lobby move packet length".to_string());
        }

        Ok(PacketData::LobbyMove(LobbyMoveData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            player_id: packet[1].as_str().unwrap_or("").to_string(),
            code: packet[2].as_str().unwrap_or("").to_string(),
            session_id: packet[3].as_str().unwrap_or("").to_string(),
            state: packet[4].as_i64().unwrap_or(0) as i32,
        }))
    }

    fn parse_hit(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 7 {
            return Err("Invalid hit packet length".to_string());


        }

        let extra = packet[5].as_object().ok_or("Invalid extra data")?;
        
        Ok(PacketData::Hit(HitData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            attacker_id: packet[1].as_i64().unwrap_or(0) as i32,
            victim_id: packet[2].as_i64().unwrap_or(0) as i32,
            damage: packet[3].as_f64().unwrap_or(0.0),
            effects: packet[4].clone(),
            hit_info: HitInfo {
                distance: extra.get("dst").and_then(|v| v.as_f64()).unwrap_or(0.0),
                is_headshot: extra.get("hs").and_then(|v| v.as_bool()).unwrap_or(false),
                is_wallbang: extra.get("wb").and_then(|v| v.as_bool()).unwrap_or(false),
                weapon_id: extra.get("wId").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            },
            meta: packet[6].clone(),
        }))
    }

    fn parse_chat(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 6 {
            return Err("Invalid chat packet length".to_string());
        }

        let message_data: &[Value] = match packet[3].as_array() {
            Some(array) => array.as_slice(),
            None => &[],
        };
        let msg_key = message_data.get(0).and_then(|v| v.as_str()).unwrap_or("");
        let player_name = message_data.get(1).and_then(|v| v.as_str()).unwrap_or("");

        Ok(PacketData::Chat(ChatData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            category: packet[1].as_i64().unwrap_or(0) as i32,
            message: msg_key.to_string(),
            player: player_name.to_string(),
            priority: packet[4].as_i64().unwrap_or(0) as i32,
            meta: packet[5].clone(),
            meaning: format!("{} triggered {}", player_name, msg_key),
        }))
    }

    fn parse_game_state(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 2 {
            return Err("Invalid game state packet length".to_string());
        }

        let state_code = packet[1].as_i64().unwrap_or(0) as i32;
        let meaning = match state_code {
            0 => "Game Ended",
            1 => "Pre-game",
            2 => "Warm-up", 
            3 => "Countdown",
            4 => "Game In Progress",
            _ => "Unknown",
        };

        Ok(PacketData::GameState(GameStateData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            state_code,
            meaning: meaning.to_string(),
        }))
    }

    fn parse_player_sync(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 2 {
            return Err("Invalid player sync packet length".to_string());
        }

        let data = packet[1].as_array().ok_or("Invalid player sync data")?;
        if data.len() < 20 {
            return Err("Insufficient player sync data".to_string());
        }

        Ok(PacketData::PlayerSync(PlayerSyncData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            id: data[0].as_i64().unwrap_or(0) as i32,
            team: data[1].as_i64().unwrap_or(0) as i32,
            position: Position {
                x: data[2].as_f64().unwrap_or(0.0),
                y: data[3].as_f64().unwrap_or(0.0),
                z: data[4].as_f64().unwrap_or(0.0),
            },
            velocity: Position {
                x: data[5].as_f64().unwrap_or(0.0),
                y: data[6].as_f64().unwrap_or(0.0),
                z: data[7].as_f64().unwrap_or(0.0),
            },
            rotation: data[8].as_f64().unwrap_or(0.0),
            flags: PlayerFlags {
                flag1: data.get(9).cloned().unwrap_or(Value::Null),
                flag2: data.get(10).cloned().unwrap_or(Value::Null),
                flag3: data.get(11).cloned().unwrap_or(Value::Null),
                flag4: data.get(12).cloned().unwrap_or(Value::Null),
                flag5: data.get(13).cloned().unwrap_or(Value::Null),
                flag6: data.get(14).cloned().unwrap_or(Value::Null),
                flag7: data.get(15).cloned().unwrap_or(Value::Null),
                flag8: data.get(16).cloned().unwrap_or(Value::Null),
                flag9: data.get(17).cloned().unwrap_or(Value::Null),
                flag10: data.get(18).cloned().unwrap_or(Value::Null),
                flag11: data.get(19).cloned().unwrap_or(Value::Null),
                flag12: data.get(20).cloned().unwrap_or(Value::Null),
                flag13: data.get(21).cloned().unwrap_or(Value::Null),
                flag14: data.get(22).cloned().unwrap_or(Value::Null),
                flag15: data.get(23).cloned().unwrap_or(Value::Null),
                flag16: data.get(24).cloned().unwrap_or(Value::Null),
            },
        }))
    }

    fn parse_player_init(&self, packet: &[Value]) -> Result<PacketData, String> {
        if packet.len() < 3 {
            return Err("Invalid player init packet length".to_string());
        }

        let data = packet[1].as_array().ok_or("Invalid player init data")?;
        if data.len() < 24 {
            return Err("Insufficient player init data".to_string());
        }

        Ok(PacketData::PlayerInit(PlayerInitData {
            r#type: packet[0].as_str().unwrap_or("").to_string(),
            uid: data[0].as_i64().unwrap_or(0) as i32,
            class_id: data[1].as_i64().unwrap_or(0) as i32,
            score: data[2].as_i64().unwrap_or(0) as i32,
            position: Position2D {
                x: data[3].as_f64().unwrap_or(0.0),
                y: data[4].as_f64().unwrap_or(0.0),
            },
            username: data[5].as_str().unwrap_or("").to_string(),
            alive: data[6].as_bool().unwrap_or(false),
            health: data[7].as_f64().unwrap_or(0.0),
            max_health: data[8].as_f64().unwrap_or(0.0),
            clan: data.get(11).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            angle: data.get(18).and_then(|v| v.as_f64()).unwrap_or(0.0),
            game_mode: data.get(20).and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            weapon_id: data.get(23).and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            raw: data.clone(),
            broadcast_flag: packet.get(2).cloned().unwrap_or(Value::Null),
        }))
    }
}

pub fn process_and_save_packet(raw_json: &str, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed: Response = serde_json::from_str(raw_json).map_err(|e| {
        eprintln!("Failed to parse JSON: {}", e);
        eprintln!("Raw JSON was: {}", raw_json);
        e
    })?;
    
    let payload = &parsed.response.payload_data;
    
    let bytes = base64::decode(payload).map_err(|e| {
        eprintln!("Failed to decode base64: {}", e);
        e
    })?;

    // decode as messagepack
    let packet_data: Vec<Value> = rmp_serde::from_slice(&bytes).or_else(|e| {
        eprintln!("Failed to decode MessagePack: {}", e);
        for i in (1..bytes.len()).rev() {
            // if fails, use js version
            if let Ok(data) = rmp_serde::from_slice::<Vec<Value>>(&bytes[..i]) {
                eprintln!("Trimmed to {} bytes to decode", i);
                return Ok(data);
            }
        }
        Err(e)
    })?;

    let opcode = match packet_data.get(0).and_then(|v| v.as_str()) {
        Some(op) => op,
        None => {
            // assume empty / wrong format
            return Ok(());
        }
    };

    // check opcode, if thats what we want or not
    if opcode != "6" {
        return Ok(());
    }
    
    let parser = PacketParser::new();
    let parsed_packet = parser.parse_packet(&packet_data)?;
    
    let json_output = serde_json::to_string_pretty(&parsed_packet)?;
    print!("{json_output}");
    
    Ok(())
}

#[derive(Deserialize)]
struct Response {
    response: ResponseData,
}

#[derive(Deserialize)]
struct ResponseData {
    #[serde(rename = "payloadData")]
    payload_data: String,
}