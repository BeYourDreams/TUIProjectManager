use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 一个保存的项目条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub hidden: bool,
}

fn default_dark_mode() -> bool {
    // 首次运行（无配置文件）时检测 Windows 系统主题偏好。
    // 注册表 AppsUseLightTheme：1=浅色（false），0=深色（true），读取失败兜底深色。
    #[cfg(target_os = "windows")]
    {
        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn RegOpenKeyExW(
                hkey: isize,
                lp_subkey: *const u16,
                ul_options: u32,
                sam_desired: u32,
                phk_result: *mut isize,
            ) -> i32;
            fn RegQueryValueExW(
                hkey: isize,
                lp_value_name: *const u16,
                lp_reserved: *mut u32,
                lp_type: *mut u32,
                lp_data: *mut u8,
                lpcb_data: *mut u32,
            ) -> i32;
            fn RegCloseKey(hkey: isize) -> i32;
        }
        const HKEY_CURRENT_USER: isize = 0x8000_0001;
        const KEY_READ: u32 = 0x0002_0019;
        const REG_DWORD: u32 = 4;
        let key_path: Vec<u16> = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let val_name: Vec<u16> = "AppsUseLightTheme"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut hkey: isize = 0;
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_READ, &mut hkey) } == 0 {
            let mut dtype: u32 = 0;
            let mut data: u32 = 0;
            let mut size: u32 = std::mem::size_of::<u32>() as u32;
            let hr = unsafe {
                RegQueryValueExW(
                    hkey,
                    val_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut dtype,
                    &mut data as *mut u32 as *mut u8,
                    &mut size,
                )
            };
            unsafe { RegCloseKey(hkey); }
            if hr == 0 && dtype == REG_DWORD {
                return data == 0; // AppsUseLightTheme=0 → 深色
            }
        }
    }
    true // 读取失败兜底深色
}

fn default_follow_system() -> bool {
    false
}

/// 终端回看历史行数上限：默认 500（每页签约 -1.9MB，见 session.rs）。
/// 回看依赖终端自身 scrollback 操作，与渲染缓存无关。
/// 默认 1000 → 500（每页签省 ~1.9MB，见 session.rs）。
fn default_history_lines() -> u32 {
    500
}

/// 工具更新：找不到时是否再扫 PATH（默认开，兼容全局安装那份）。
fn default_true() -> bool {
    true
}
fn default_done_quiet_ms() -> u64 {
    8_000
}

/// 页签「✅ 判完成」的静默门槛的合法区间（毫秒）。
///
/// 下界 1s：低于 1s 时屏幕任何一格的变化都会被立刻当成“在跑”，而命令输出
/// 的块间静默（PTY 一块读、TUI 重绘间隔）轻易就超过 1s → 🔄 常驻、✅ 几乎
/// 永不亮。上界 5 分钟：再长就不是“延迟”，而是用户根本拿不到完成信号。
pub const DONE_QUIET_MIN_MS: u64 = 1_000;
pub const DONE_QUIET_MAX_MS: u64 = 300_000;

/// 把配置里（或手改过的）静默门槛夹回合法区间：越界值不报错、直接夹住。
/// 手改 config.json 写成 0 / 999999 时要么 ✅ 永不亮、要么迟得没法用，
/// 夹住比让整份配置解析失败或状态机失控都好。
pub fn clamp_done_quiet_ms(v: u64) -> u64 {
    v.clamp(DONE_QUIET_MIN_MS, DONE_QUIET_MAX_MS)
}


/// → 取路径末段文件名 → 去 `.exe` 后缀（不区分大小写）→ 折叠空白 → 小写。
/// 于是 `nvim`、`NVIM`、`nvim.exe`、`D:\Tools\nvim.EXE`、`"nvim"` 视为同一条，
/// 添加 / 改名时用它比对，不会把同一条命令重复塞进列表。
pub fn tui_command_key(cmd: &str) -> String {
    let c = cmd.trim().trim_matches(|ch| ch == '"' || ch == '\'').trim();
    // 取命令本身：路径写法（带分隔符）取最后一段分隔符之后的部分，否则取第一个
    // token——`C:\Program Files\Git\bin\bash.exe -l` 这种带空格的路径才不会
    // 被截成 "C:\Program"。
    let tail = match c.rfind(['\\', '/']) {
        Some(i) => &c[i + 1..],
        None => c,
    };
    let base = tail.split_whitespace().next().unwrap_or(tail);
    let stem = if base.len() > 4 && base[base.len() - 4..].eq_ignore_ascii_case(".exe") {
        &base[..base.len() - 4]
    } else {
        base
    };
    stem.trim().to_lowercase()
}

/// 程序设置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    /// 已配置的 TUI 命令列表。
    #[serde(default)]
    pub tui_commands: Vec<String>,
    /// 当前选中的 TUI 命令（启动项目时使用）。
    #[serde(default)]
    pub tui_command: String,
    /// 深浅主题：true=深色（默认），false=浅色。
    #[serde(default = "default_dark_mode")]
    pub dark_mode: bool,
    /// 跟随系统主题：true 时按系统深浅动态切换（覆盖 dark_mode）。
    #[serde(default = "default_follow_system")]
    pub follow_system: bool,
    /// 终端回看历史行数上限（100..=5000）。默认 500。
    #[serde(default = "default_history_lines")]
    pub history_lines: u32,
    /// 「检查更新」里 pi / opencode 的查找位置（**本机路径，不跨机器共用**）：
    /// 每项可以是 exe 完整路径（`D:\soft\TUIProjectManager\pi.exe`）或所在
    /// 目录（目录则同时试 `pi.exe` 与 `pi\pi.exe`）。启动时自动补齐本软件同级
    /// 目录下的 pi / opencode 路径，用户可在设置页增删（换机器就改这里）。
    #[serde(default)]
    pub tool_paths: Vec<String>,
/// 上面都找不到时，是否再扫 PATH。默认 true。
    #[serde(default = "default_true")]
    pub tool_search_path: bool,
    /// 首页项目排序方式（记住上次选择）。
    #[serde(default)]
    pub project_sort: ProjectSort,
    /// 首页是否显示隐藏项目（记住上次选择）。
    #[serde(default)]
    pub show_hidden: bool,
    /// 提醒方式：系统 toast 通知 / 任务栏闪烁（可都开关；两个都关 = 只有页签 ✅）。
    #[serde(default = "default_true")]
    pub notify_toast: bool,
    #[serde(default = "default_true")]
    pub notify_flash: bool,

    /// 页签「判完成」的延迟静默门槛（毫秒）：屏幕**任一可见格**不变满此时长
    /// 才判完成、✅ 才亮起。可在设置页调（[`DONE_QUIET_MIN_MS`]..=[`DONE_QUIET_MAX_MS`]）。
    /// 调大 = 少谎报完成、✅ 更晚亮；调小 = ✅ 更快亮、更易误判。
    #[serde(default = "default_done_quiet_ms")]
    pub done_quiet_ms: u64,
}

/// 首页项目列表排序模式。升/降序仅作用于显示层（按名称首字），不修改
/// config 里的原始数据；「默认顺序」展示原始数据顺序，自定义排序通过拖拽
/// 重排完成（会持久化修改原始数据）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSort {
    /// 原始数据顺序（可拖拽自定义重排）。
    #[default]
    Default,
    /// 按名称首字升序，仅显示层排序。
    NameAsc,
    /// 按名称首字降序，仅显示层排序。
    NameDesc,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tui_commands: vec!["nvim".to_string()],
            tui_command: "nvim".to_string(),
            dark_mode: true,
            follow_system: false,
            history_lines: default_history_lines(),
tool_paths: Vec::new(),
            tool_search_path: true,
            project_sort: ProjectSort::default(),
            show_hidden: false,
            notify_toast: true,
            notify_flash: true,

            done_quiet_ms: default_done_quiet_ms(),
        }
    }
}

/// 应用配置，保存到与程序同级目录下的 config/config.json。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub projects: Vec<Project>,
    pub settings: Settings,
    /// 窗口位置/大小，下次启动时恢复。
    #[serde(default)]
    pub window: WindowState,
    /// 上次打开中的页签，下次启动时重新拉起。
    #[serde(default)]
    pub tabs: TabsState,
}

/// 上次的窗口状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct WindowState {
    pub pos: Option<[f32; 2]>,
    pub size: Option<[f32; 2]>,
    pub maximized: bool,
}

/// 上次退出时打开中的终端页签（启动时重新拉起）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct TabsState {
    /// 打开的会话目录，按页签顺序。
    pub dirs: Vec<String>,
    /// 各页签退出时使用的命令（与 dirs 一一对应，旧配置缺省时 fallback 到 tui_command）。
    #[serde(default)]
    pub cmds: Vec<String>,
    /// 上次激活的页签索引（满页签栏索引，0 = 首页）。
    pub active: usize,
    /// 上次退出时设置页签是否打开。
    #[serde(default)]
    pub settings_open: bool,
/// 设置页签在页签栏中的位置。**已固定为 1（首页之后）**，字段保留只为读旧配置。
    #[serde(default)]
    pub settings_pos: usize,
}

/// 与程序可执行文件同级的 config 目录。
pub fn config_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        return dir.join("config");
    }
    PathBuf::from("config")
}

/// 配置文件路径。
pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// 加载配置；文件不存在或解析失败时返回默认配置。
pub fn load() -> Config {
    load_from(&config_dir())
}

/// 从指定配置目录加载（测试与 `save_to` 配对用；`load` 走 exe 同级 config/）。
///
/// 解析失败不静默清零：把坏文件改名成 `config.json.bad` 留证据再退默认。
/// 否则下一次自动保存就把用户唯一那份配置盖掉——断电/死机后残留半截 JSON
/// （或改名已落盘、内容还在系统缓存里）的典型后果是全量丢项目。
pub fn load_from(dir: &std::path::Path) -> Config {
    let path = dir.join("config.json");
    let mut config = match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str::<Config>(&raw) {
            Ok(c) => c,
            Err(_) => {
                let bad = path.with_extension("json.bad");
                let _ = std::fs::remove_file(&bad);
                let _ = std::fs::rename(&path, &bad);
                Config::default()
            }
        },
        Err(_) => Config::default(),
    };
    sanitize_settings(&mut config.settings);
    // 旧版本只有 tui_command，迁移到 tui_commands 列表。
    if config.settings.tui_commands.is_empty() {
        let cmd = config.settings.tui_command.trim().to_string();
        if !cmd.is_empty() {
            config.settings.tui_commands.push(cmd);
        }
    }
    normalize_tui_commands(&mut config.settings);
    config
}

/// 把越界的数值型设置夹回合法区间：手改 config.json / 老版本写下的值都可能
/// 出界（`done_quiet_ms: 0` 会让 ✅ 永不亮），逐项夹住而不是让状态机拿到
/// 一个语义上不存在的数。
fn sanitize_settings(s: &mut Settings) {
    s.done_quiet_ms = clamp_done_quiet_ms(s.done_quiet_ms);
}

/// 启动命令列表去重（按 tui_command_key 等价判重，保留首次出现的写法）：
/// 配置被手改过 / 旧版本迁移都可能留下 `nvim` + `nvim.exe` 这类重复项，
/// 重复项会让启动命令选择出现两个同义项。顺带修正指向已被去掉的
/// `tui_command`（回落到第一条）。
pub fn normalize_tui_commands(s: &mut Settings) {
    let mut seen: Vec<String> = Vec::new();
    s.tui_commands.retain(|c| {
        let k = tui_command_key(c);
        if k.is_empty() || seen.contains(&k) {
            return false;
        }
        seen.push(k);
        true
    });
    // 选中项归一到列表里真实存在的那条（等价的换写法也改写成列表里那条），
    // 否则回落到第一条；本来就空的不动。
    let key = tui_command_key(&s.tui_command);
    if !key.is_empty() {
        s.tui_command = s
            .tui_commands
            .iter()
            .find(|c| tui_command_key(c) == key)
            .cloned()
            .or_else(|| s.tui_commands.first().cloned())
            .unwrap_or_default();
    }
}

/// 保存配置到 config/config.json。
pub fn save(config: &Config) -> Result<(), String> {
    save_to(&config_dir(), config)
}

/// 写配置文件到指定目录（`save` 走 exe 同级 config/）。
pub fn save_to(dir: &std::path::Path, config: &Config) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    // 先写临时文件、fsync 再 rename：进程崩溃只会留 config.json.tmp，不会留半截
    // config.json；fsync 防的是断电/死机——rename 只保证目录项落盘，内容还在系统
    // 写缓存里时掉电会得到一个 0 字节或半截的文件（真丢整份配置）。
    let tmp = dir.join("config.json.tmp");
    {
        use std::io::Write as _;
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, dir.join("config.json")).map_err(|e| e.to_string())
}

// ── 模型配置（pi / oh-my-pi） ──────────────────────────────────────────

/// 单个模型条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub max_tokens: u64,
}

impl Default for ModelEntry {
    fn default() -> Self {
        Self {
            id: "1".into(),
            name: "1".into(),
            context_window: 128_000,
            max_tokens: 16_384,
        }
    }
}

/// 单个 provider 条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderEntry {
    pub base_url: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<ModelEntry>,
}

impl Default for ProviderEntry {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:20128/v1".into(),
            api: "openai-completions".into(),
            api_key: "sk-18d904d21da7b328-s15pfo-8f0e32c3".into(),
            models: vec![ModelEntry::default()],
        }
    }
}

/// 模型配置（pi 的 JSON / oh-my-pi 的 YAML 共用此结构）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelsConfig {
    #[serde(default)]
    pub providers: std::collections::HashMap<String, ProviderEntry>,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        let mut providers = std::collections::HashMap::new();
        providers.insert("1".into(), ProviderEntry::default());
        Self { providers }
    }
}

/// 用户 home 目录。
fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// pi 模型配置文件路径：~/.pi/agent/models.json
pub fn pi_models_path() -> PathBuf {
    home_dir().join(".pi").join("agent").join("models.json")
}

/// oh-my-pi 模型配置文件路径：~/.omp/agent/models.yml
pub fn omp_models_path() -> PathBuf {
    home_dir().join(".omp").join("agent").join("models.yml")
}

// ── opencode 供应商配置（~/.config/opencode/opencode.json） ───────────────

/// opencode 的模型条目（UI 形态；on-disk 是 `models.<id>.name` 的 map）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OcModel {
    pub id: String,
    pub name: String,
}

/// opencode 的供应商条目（UI 形态；on-disk 是 provider 的一个键值）。
/// 字段名与 opencode 自己的 schema 对齐（`npm` / `options.baseURL`），
/// 和 pi 那套 `baseUrl` + models 数组**不通用**，故单列一组类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcProvider {
    /// on-disk 里 provider 的键（引用模型时形如 `<id>/<model>`）。
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// AI SDK 包名，本地 OpenAI 兼容端点用 `@ai-sdk/openai-compatible`。
    #[serde(default = "default_oc_npm")]
    pub npm: String,
    #[serde(default = "default_oc_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<OcModel>,
}

fn default_oc_npm() -> String {
    "@ai-sdk/openai-compatible".to_string()
}

fn default_oc_base_url() -> String {
    "http://localhost:20128/v1".to_string()
}

impl Default for OcProvider {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            npm: default_oc_npm(),
            base_url: default_oc_base_url(),
            api_key: String::new(),
            models: vec![OcModel {
                id: "1".into(),
                name: "1".into(),
            }],
        }
    }
}

/// opencode 供应商集合（UI 侧一份）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OcProviders {
    pub providers: Vec<OcProvider>,
}

/// opencode 配置目录：%XDG_CONFIG_HOME%/opencode（未设则 ~/.config/opencode）。
pub fn opencode_config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("opencode")
}

/// opencode 配置文件路径：优先已存在的 opencode.jsonc / opencode.json，
/// 都没有则用 opencode.json（写入时才创建）。
pub fn opencode_config_path() -> PathBuf {
    let dir = opencode_config_dir();
    for name in ["opencode.jsonc", "opencode.json"] {
        let p = dir.join(name);
        if p.is_file() {
            return p;
        }
    }
    dir.join("opencode.json")
}

/// opencode 顶层 `model`（形如 `<供应商>/<模型>`，即默认模型）：只读展示，
/// 本程序不修改（改了还得同步 `disabled_providers` 等字段，交给用户自己定）。
pub fn opencode_default_model() -> String {
    let Ok(raw) = std::fs::read_to_string(opencode_config_path()) else {
        return String::new();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return String::new();
    };
    doc.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// 读取 opencode 的 provider 列表。
///
/// 整份文件当 `serde_json::Value` 读，只把 `provider` 子树转成 UI 结构；
/// **解析失败（典型是 opencode.jsonc 里写了注释）直接返回 Err 且不落盘**——
/// 宁可让用户手改，也不能把注释或本程序不认识的字段（`model` / `agent` /
/// `mcp` / `disabled_providers` …）洗掉。
pub fn load_opencode_providers() -> Result<OcProviders, String> {
    load_opencode_providers_from(&opencode_config_path())
}

/// 指定路径版（单测用；见 load_opencode_providers 的说明）。
pub fn load_opencode_providers_from(path: &std::path::Path) -> Result<OcProviders, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        // 没有配置文件 = 空白起点（写回时才会创建文件）。
        Err(_) => return Ok(OcProviders::default()),
    };
    let doc: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut cfg = OcProviders::default();
    let Some(provs) = doc.get("provider").and_then(|v| v.as_object()) else {
        return Ok(cfg); // 没有 provider 段（或不是对象）：当空表单，不动原文件
    };
    for (id, v) in provs {
        // 不用 Default::default() 的预置模型行：on-disk 有几个就显示几个。
        let mut p = OcProvider {
            id: id.clone(),
            models: Vec::new(),
            ..Default::default()
        };
        if let Some(s) = v.get("name").and_then(|x| x.as_str()) {
            p.name = s.to_string();
        }
        if let Some(s) = v.get("npm").and_then(|x| x.as_str()) {
            p.npm = s.to_string();
        }
        if let Some(s) = v
            .get("options")
            .and_then(|o| o.get("baseURL"))
            .and_then(|x| x.as_str())
        {
            p.base_url = s.to_string();
        }
        if let Some(s) = v
            .get("options")
            .and_then(|o| o.get("apiKey"))
            .and_then(|x| x.as_str())
        {
            p.api_key = s.to_string();
        }
        if let Some(ms) = v.get("models").and_then(|x| x.as_object()) {
            for (mid, mv) in ms {
                p.models.push(OcModel {
                    id: mid.clone(),
                    name: mv
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or(mid.as_str())
                        .to_string(),
                });
            }
        }
        cfg.providers.push(p);
    }
    Ok(cfg)
}

/// 写回 opencode 配置：只 patch 我们管理的字段（provider 的 `name` / `npm` /
/// `options.baseURL` / `options.apiKey` / `models.<id>.name`）与界面上被删掉的
/// 那几项，其余键原样保留。
pub fn save_opencode_providers(cfg: &OcProviders) -> Result<(), String> {
    save_opencode_providers_to(&opencode_config_path(), cfg)
}

/// 指定路径版（单测用）：真正的写回逻辑，路径参数化便于验证。
pub fn save_opencode_providers_to(path: &std::path::Path, cfg: &OcProviders) -> Result<(), String> {
    let mut doc: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?,
        Err(_) => serde_json::json!({ "$schema": "https://opencode.ai/config.json" }),
    };
    if !doc.is_object() {
        return Err(format!("{} 不是 JSON 对象，已跳过写入", path.display()));
    }
    let root = doc.as_object_mut().unwrap();
    if !root.contains_key("provider") {
        root.insert("provider".into(), serde_json::json!({}));
    }
    let provs = root.get_mut("provider").unwrap();
    if !provs.is_object() {
        *provs = serde_json::json!({});
    }
    let provs = provs.as_object_mut().unwrap();
    for p in &cfg.providers {
        let key = p.id.trim();
        if key.is_empty() {
            continue; // 空 id 不写
        }
        if !provs.contains_key(key) {
            provs.insert(key.to_string(), serde_json::json!({}));
        }
        let entry = provs.get_mut(key).unwrap();
        if !entry.is_object() {
            *entry = serde_json::json!({});
        }
        let e = entry.as_object_mut().unwrap();
        let name = if p.name.trim().is_empty() { key } else { p.name.trim() };
        e.insert("name".into(), serde_json::json!(name));
        if !p.npm.trim().is_empty() {
            e.insert("npm".into(), serde_json::json!(p.npm.trim()));
        }
        if !e.contains_key("options") {
            e.insert("options".into(), serde_json::json!({}));
        }
        let opts = e.get_mut("options").unwrap();
        if !opts.is_object() {
            *opts = serde_json::json!({});
        }
        let o = opts.as_object_mut().unwrap();
        o.insert("baseURL".into(), serde_json::json!(p.base_url.trim()));
        o.insert("apiKey".into(), serde_json::json!(p.api_key.trim()));
        if !e.contains_key("models") {
            e.insert("models".into(), serde_json::json!({}));
        }
        let ms = e.get_mut("models").unwrap();
        if !ms.is_object() {
            *ms = serde_json::json!({});
        }
        let ms = ms.as_object_mut().unwrap();
        for m in &p.models {
            let mid = m.id.trim();
            if mid.is_empty() {
                continue;
            }
            if !ms.contains_key(mid) {
                ms.insert(mid.to_string(), serde_json::json!({}));
            }
            let mv = ms.get_mut(mid).unwrap();
            if !mv.is_object() {
                *mv = serde_json::json!({});
            }
            // 只改 name，模型项里的 limit/reasoning 等本程序不碰的字段保留。
            mv.as_object_mut()
                .unwrap()
                .insert("name".into(), serde_json::json!(m.name.trim()));
        }
        // 界面上删掉的模型：按 id 移除（不碰其他模型项内部的字段）。
        let live: Vec<String> = cfg
            .providers
            .iter()
            .find(|q| q.id.trim() == key)
            .map(|q| q.models.iter().map(|m| m.id.trim().to_string()).collect())
            .unwrap_or_default();
        ms.retain(|k, _| live.iter().any(|l| l == k));
    }
    // 界面上删掉的供应商：整键移除。
    let live: Vec<String> = cfg
        .providers
        .iter()
        .map(|p| p.id.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    provs.retain(|k, _| live.iter().any(|l| l == k));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut json = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    json.push('\n');
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// 读取 pi 模型配置；文件不存在时创建默认配置。
pub fn load_pi_models() -> ModelsConfig {
    let path = pi_models_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => {
            let cfg = ModelsConfig::default();
            let _ = save_pi_models(&cfg);
            cfg
        }
    }
}

/// 保存 pi 模型配置。
pub fn save_pi_models(cfg: &ModelsConfig) -> Result<(), String> {
    let path = pi_models_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// 读取 oh-my-pi 模型配置；文件不存在时创建默认配置。
pub fn load_omp_models() -> ModelsConfig {
    let path = omp_models_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_yaml::from_str(&raw).unwrap_or_default(),
        Err(_) => {
            let cfg = ModelsConfig::default();
            let _ = save_omp_models(&cfg);
            cfg
        }
    }
}

/// 保存 oh-my-pi 模型配置。
pub fn save_omp_models(cfg: &ModelsConfig) -> Result<(), String> {
    let path = omp_models_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let yaml = serde_yaml::to_string(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, yaml).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧配置文件里的 `ctrl_shift_z` 字段（Ctrl+Shift+Z 翻译，已整体移除）不该
    /// 加载失败：Settings 没开 deny_unknown_fields，未知字段直接忽略。
    #[test]
    fn legacy_ctrl_shift_z_field_ignored() {
        let s: Settings = serde_json::from_str(
            r#"{"tui_commands":["nvim"],"tui_command":"nvim","ctrl_shift_z":"^Y"}"#,
        )
        .unwrap();
        assert_eq!(s.tui_command, "nvim");
    }

    #[test]
    fn models_json_roundtrip() {
        let cfg = ModelsConfig::default();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        // 验证字段名是 camelCase
        assert!(json.contains("baseUrl"), "JSON 应含 camelCase baseUrl: {json}");
        assert!(json.contains("apiKey"), "JSON 应含 camelCase apiKey: {json}");
        assert!(json.contains("contextWindow"), "JSON 应含 camelCase contextWindow: {json}");
        assert!(json.contains("maxTokens"), "JSON 应含 camelCase maxTokens: {json}");
        // 回环验证
        let parsed: ModelsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn models_yaml_roundtrip() {
        let cfg = ModelsConfig::default();
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        assert!(yaml.contains("baseUrl"), "YAML 应含 camelCase baseUrl: {yaml}");
        assert!(yaml.contains("apiKey"), "YAML 应含 camelCase apiKey: {yaml}");
        assert!(yaml.contains("contextWindow"), "YAML 应含 camelCase contextWindow: {yaml}");
        assert!(yaml.contains("maxTokens"), "YAML 应含 camelCase maxTokens: {yaml}");
        let parsed: ModelsConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed, cfg);
    }

    /// 启动命令等价键：大小写、`.exe` 后缀、路径写法、引号、带参数都归一。
    #[test]
    fn tui_command_key_normalizes_forms() {
        let k = tui_command_key;
        assert_eq!(k("nvim"), k("NVIM"));
        assert_eq!(k("nvim"), k("nvim.exe"));
        assert_eq!(k("nvim"), k("NVIM.EXE"));
        assert_eq!(k("nvim"), k(r#"D:\Tools\nvim.exe"#));
        assert_eq!(k("nvim"), k(r#""nvim""#));
        assert_eq!(k("nvim"), k("  nvim  "));
        assert_eq!(k("nvim"), k("nvim -u NONE")); // 参数不参与比较
        assert_eq!(k("bash"), k(r"C:\Program Files\Git\bin\bash.exe"));
        // 不同命令不能被归一成同一条。
        assert_ne!(k("nvim"), k("vim"));
        assert_ne!(k("lazygit"), k("git"));
        assert_eq!(k("   "), "");
    }

    /// 加载配置时的去重：同义写法只留首条，指向被去掉项/换写法的选中命令
    /// 归一到列表里真实存在的那条。
    #[test]
    fn normalize_tui_commands_dedups_and_fixes_selection() {
        let mut s = Settings {
            tui_commands: vec![
                "nvim".into(),
                "NVIM.EXE".into(),
                r"D:\Tools\nvim.exe".into(),
                "lazygit".into(),
                "  ".into(),
            ],
            tui_command: "nvim.exe".into(),
            ..Settings::default()
        };
        normalize_tui_commands(&mut s);
        assert_eq!(s.tui_commands, vec!["nvim".to_string(), "lazygit".to_string()]);
        // 选中的 nvim.exe 被去重掉了，但与 nvim 等价 → 保留指向（改写为列表里那条）。
        assert_eq!(s.tui_command, "nvim");
        // 选中项完全不在列表里 → 回落到第一条。
        let mut s2 = Settings {
            tui_commands: vec!["lazygit".into(), "htop".into()],
            tui_command: "nvim".into(),
            ..Settings::default()
        };
        normalize_tui_commands(&mut s2);
        assert_eq!(s2.tui_command, "lazygit");
    }

    /// opencode 写回只 patch provider 子树：
    /// 里的额外字段 / 模型项的 `limit` 都得原样保留；删掉的供应商与模型消失；
    /// 键名用 opencode 自己的 `options.baseURL`（不是 pi 的 `baseUrl`）。
    #[test]
    fn opencode_save_patches_only_provider_subtree() {
        let path = std::env::temp_dir().join("tpm_test_opencode.json");
        std::fs::write(
            &path,
            r#"{
  "$schema": "https://opencode.ai/config.json",
  "disabled_providers": [],
  "model": "1/1",
  "provider": {
    "1": {
      "name": "1",
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "http://old/v1", "apiKey": "sk-old", "extraOpt": 1 },
      "models": { "1": { "name": "old", "limit": { "context": 128000 } }, "2": { "name": "drop-me" } }
    },
    "gone": { "npm": "x", "options": {} }
  }
}"#,
        )
        .unwrap();
        // 读出来是 UI 形态（供应商 id / 模型 id 与 on-disk 的键一致）。
        let mut cfg = load_opencode_providers_from(&path).unwrap();
        assert_eq!(cfg.providers.len(), 2);
        let p = cfg.providers.iter_mut().find(|p| p.id == "1").unwrap();
        assert_eq!(p.base_url, "http://old/v1");
        assert_eq!(p.api_key, "sk-old");
        assert_eq!(p.models.len(), 2);
        p.name = "本地".into();
        p.base_url = "http://localhost:20128/v1".into();
        p.api_key = "sk-new".into();
        p.models[0].name = "DeepSeek V4 Flash".into();
        p.models.pop(); // 删掉模型 2
        cfg.providers.retain(|p| p.id != "gone"); // 删掉供应商 gone
        save_opencode_providers_to(&path, &cfg).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("opencode.ai/config.json"), "$schema 应保留: {out}");
        assert!(out.contains("disabled_providers"), "disabled_providers 应保留: {out}");
        assert!(out.contains("\"model\": \"1/1\""), "顶层 model 应保留: {out}");
        assert!(out.contains("extraOpt"), "options 未知字段应保留: {out}");
        assert!(out.contains("\"context\": 128000"), "模型 limit 应保留: {out}");
        assert!(out.contains("DeepSeek V4 Flash"), "模型名应更新: {out}");
        assert!(out.contains("baseURL"), "应使用 opencode 的 baseURL 键: {out}");
        assert!(!out.contains("baseUrl\""), "不该写出 pi 的 baseUrl: {out}");
        assert!(!out.contains("drop-me"), "删掉的模型应消失: {out}");
        assert!(!out.contains("\"gone\""), "删掉的供应商应消失: {out}");
        // 回环：再读一次，改动确实落到了 provider 子树。
        let back = load_opencode_providers_from(&path).unwrap();
        let p = back.providers.iter().find(|p| p.id == "1").unwrap();
        assert_eq!(p.base_url, "http://localhost:20128/v1");
        assert_eq!(p.api_key, "sk-new");
        assert_eq!(p.models.len(), 1);
        assert_eq!(p.models[0].name, "DeepSeek V4 Flash");
        // 带注释的 jsonc 解析失败 → 报错而不是静默洗掉注释。
        let bad = std::env::temp_dir().join("tpm_test_opencode.jsonc");
        std::fs::write(&bad, "{\n // 注释\n \"provider\": {}\n}").unwrap();
        assert!(load_opencode_providers_from(&bad).is_err());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&bad);
        // 默认值与 pi 侧本地网关一致。
        let d = OcProvider::default();
        assert_eq!(d.npm, "@ai-sdk/openai-compatible");
assert_eq!(d.base_url, "http://localhost:20128/v1");
    }

    /// 项目排序与「显示隐藏项目」跨重启保留；旧配置没这两字段时回落默认值。
    #[test]
    fn settings_round_trip_project_sort_and_show_hidden() {
        let mut s = Settings::default();
        s.project_sort = ProjectSort::NameDesc;
        s.show_hidden = true;
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project_sort, ProjectSort::NameDesc);
        assert!(back.show_hidden);
        let old: Settings = serde_json::from_str(r#"{"tui_commands":["nvim"]}"#).unwrap();
        assert_eq!(old.project_sort, ProjectSort::Default);
        assert!(!old.show_hidden);
    }

    /// 提醒开关：默认两个都开；旧配置没这两字段时也必须是 true（否则老用户
    /// 升级后提醒莫名消失）。
    #[test]
    fn settings_round_trip_notify_switches() {
        let d = Settings::default();
        assert!(d.notify_toast && d.notify_flash);
        let mut s = Settings::default();
        s.notify_toast = false;
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert!(!back.notify_toast && back.notify_flash);
        let old: Settings = serde_json::from_str(r#"{"tui_commands":["nvim"]}"#).unwrap();
        assert!(old.notify_toast && old.notify_flash);
    }

    /// 「✅ 判完成」的静默门槛：默认 8s；旧配置没这字段时也得是 8s（否则老用户
    /// 升级后门槛变成 0，屏幕一动就 🔄、✅ 永不亮）。手改出界的值夹回区间。
    #[test]
    fn done_quiet_ms_default_and_clamped() {
        assert_eq!(Settings::default().done_quiet_ms, default_done_quiet_ms());
        let old: Settings = serde_json::from_str(r#"{"tui_commands":["nvim"]}"#).unwrap();
        assert_eq!(old.done_quiet_ms, default_done_quiet_ms());
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("done_quiet_ms"), "要落盘才能在设置页调: {json}");

        // 区间两端与越界值：0 → 下界（0 会让 ✅ 永不亮），超大 → 上界。
        assert_eq!(clamp_done_quiet_ms(0), DONE_QUIET_MIN_MS);
        assert_eq!(clamp_done_quiet_ms(DONE_QUIET_MIN_MS - 1), DONE_QUIET_MIN_MS);
        assert_eq!(clamp_done_quiet_ms(5_000), 5_000);
        assert_eq!(clamp_done_quiet_ms(DONE_QUIET_MAX_MS), DONE_QUIET_MAX_MS);
        assert_eq!(clamp_done_quiet_ms(DONE_QUIET_MAX_MS + 1), DONE_QUIET_MAX_MS);
        assert_eq!(clamp_done_quiet_ms(u64::MAX), DONE_QUIET_MAX_MS);

        // 加载路径也要夹（手改 config.json 是真实场景）。
        let dir = tmp_dir("done_quiet");
        std::fs::write(
            dir.join("config.json"),
            r#"{"settings":{"done_quiet_ms":0}}"#,
        )
        .unwrap();
        assert_eq!(load_from(&dir).settings.done_quiet_ms, DONE_QUIET_MIN_MS);
    }

    /// 每个测试一份独立临时目录
    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("tpm-cfg-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 落盘/读回不能丢页签（死机后能否恢复全靠这份列表）。
    #[test]
    fn save_then_load_keeps_tabs() {
        let dir = tmp_dir("roundtrip");
        let mut cfg = Config::default();
        cfg.tabs = TabsState {
            dirs: vec![r"D:\a".into(), r"D:\b".into()],
            cmds: vec!["pi.exe".into(), "pi.exe".into()],
            active: 2,
            settings_open: true,
            settings_pos: 1,
        };
        save_to(&dir, &cfg).unwrap();
        assert!(!dir.join("config.json.tmp").exists(), "临时文件应已被 rename 掉");
        assert_eq!(load_from(&dir).tabs, cfg.tabs);
    }

    /// 半截 JSON（断电/死机的典型残留）不许被下一次保存直接盖掉：
    /// 改名留证再退默认，否则用户唯一那份配置就没了。
    #[test]
    fn corrupt_config_is_kept_aside() {
        let dir = tmp_dir("corrupt");
        let path = dir.join("config.json");
        let raw = r#"{"projects": [ {"name": "x""#;
        std::fs::write(&path, raw).unwrap();

        assert!(load_from(&dir).projects.is_empty(), "坏配置退回默认");
        assert_eq!(
            std::fs::read_to_string(dir.join("config.json.bad")).unwrap(),
            raw
        );
        assert!(!path.exists());
    }
}
