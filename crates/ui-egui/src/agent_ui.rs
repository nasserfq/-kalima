//! وكيل (AI Agent): a side pane where the person chats with an AI model that edits the open
//! document live, like Copilot in Word. Home ▸ AI Agent (`ui.agentPane`) opens it.
//!
//! The model runs on a worker thread (`wordcraft_agent::run`). Its tools are the MCP server's own
//! (`wordcraft_mcp::tools`), but they reach this window instead of a TCP port: each call becomes a
//! [`ControlRequest`] on a channel that [`drain`] answers every frame, so every edit happens in the
//! window as the person watches. A whole turn is one Undo step ("AI Agent").
//!
//! Keys are kept in the macOS Keychain (elsewhere in an owner-only file in the settings folder)
//! and sent only to their own service. The web build has no agent: the pane says so.

use crate::WordApp;

/// Strings: Arabic in the Arabic interface, English otherwise.
fn l(ar: &'static str, en: &'static str) -> &'static str {
    if crate::i18n::current().code() == "ar" { ar } else { crate::i18n::t(en) }
}

/// The pane's title and the ribbon button's label.
pub(crate) fn title() -> &'static str {
    l("وكيل", "AI Agent")
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub(crate) open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    state: Option<native::State>,
}

/// Show or hide the pane (`ui.agentPane`, `{"value": bool}` or a toggle).
pub(crate) fn toggle(app: &mut WordApp, value: Option<bool>) {
    app.agent.open = value.unwrap_or(!app.agent.open);
}

/// The agent's own requests and its worker's news, every frame.
pub(crate) fn drain(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    native::drain(app, ctx);
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

/// The pane's body.
pub(crate) fn pane(app: &mut WordApp, ui: &mut egui::Ui) {
    #[cfg(not(target_arch = "wasm32"))]
    native::pane(app, ui);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = app;
        ui.label(l("وكيل متاح في تطبيق كلمة على الجهاز، لا في المتصفح.", "The AI Agent is available in the desktop app, not in the web browser."));
    }
}

/// Send `text` as the person's message, as if typed in the pane (`ui.agentPane` with `ask`):
/// for tests and the control channel.
pub(crate) fn ask(app: &mut WordApp, ctx: &egui::Context, text: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.agent.open = true;
        native::send(app, ctx, text.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (app, ctx, text);
        Err("the AI Agent needs the desktop app".into())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::time::Duration;

    use egui::{Align, Color32, Key, KeyboardShortcut, Layout, Modifiers, RichText, Ui};
    use serde_json::{Value, json};
    use wordcraft_agent::providers::{ModelInfo, Provider, default_model, list_models};
    use wordcraft_agent::run::{RunConfig, RunEvent, Tools};
    use wordcraft_agent::settings::Settings;
    use wordcraft_agent::types::{Message, Part, Role, ToolSpec};
    use wordcraft_agent::{Error, INVALID_INPUT, MAX_RESULT, secrets};

    use super::l;
    use crate::WordApp;
    use crate::control::{ControlRequest, Outcome};
    use crate::theme::Tokens;

    /// The tools the agent gets: reading the document and running its commands. Opening, saving
    /// and the window itself stay with the person.
    const TOOLS: &[&str] = &["get_text", "inspect_document", "list_commands", "select_text", "type_text", "execute", "batch"];

    /// How long one tool call may wait for the window (a big document's first layout is slow).
    const TOOL_TIMEOUT: Duration = Duration::from_secs(120);

    /// The services offered on the first-run card, the free one first.
    const SETUP: &[&str] = &["gemini", "openrouter", "anthropic", "groq", "ollama"];

    pub(super) struct State {
        dir: Option<PathBuf>,
        settings: Settings,
        /// Keys read so far, by provider id ("" = none saved).
        keys: HashMap<String, String>,
        chat: Vec<Message>,
        input: String,
        key_input: String,
        /// The first-run card is showing, for this provider.
        setup: Option<String>,
        run: Option<Running>,
        /// Models by provider id, once listed.
        models: HashMap<String, Vec<ModelInfo>>,
        loading: Option<(String, Receiver<Result<Vec<ModelInfo>, String>>)>,
        /// The worker's tool calls, answered in [`drain`].
        req_tx: Sender<ControlRequest>,
        req_rx: Receiver<ControlRequest>,
        /// The last error, shown under the chat.
        notice: Option<String>,
        /// Dollars spent in this chat, where the price is known.
        cost: f64,
    }

    struct Running {
        rx: Receiver<Msg>,
        cancel: Arc<AtomicBool>,
        /// The reply as it streams in.
        text: String,
        /// The tool running now: its call id.
        tool: Option<String>,
        /// Undo history position and document when the turn began.
        mark: u64,
        doc: u64,
    }

    enum Msg {
        Event(RunEvent),
        /// The turn ended: why, if it went wrong.
        Done(Option<String>),
    }

    impl State {
        fn new() -> Self {
            let dir = wordcraft_control_key::settings_dir();
            let settings = dir.as_deref().map(Settings::load).unwrap_or_default();
            let (req_tx, req_rx) = channel();
            State {
                dir,
                settings,
                keys: HashMap::new(),
                chat: Vec::new(),
                input: String::new(),
                key_input: String::new(),
                setup: None,
                run: None,
                models: HashMap::new(),
                loading: None,
                req_tx,
                req_rx,
                notice: None,
                cost: 0.0,
            }
        }

        fn provider(&self) -> Option<Provider> {
            self.settings.provider(&self.settings.provider)
        }

        fn key(&mut self, id: &str) -> String {
            if let Some(k) = self.keys.get(id) {
                return k.clone();
            }
            let k = self.dir.as_deref().map(|d| secrets::get(d, id)).unwrap_or_default();
            self.keys.insert(id.to_string(), k.clone());
            k
        }

        /// Ready to chat: a service is chosen and has the key it needs.
        fn ready(&mut self) -> bool {
            match self.provider() {
                Some(p) => !p.needs_key || !self.key(&p.id).is_empty(),
                None => false,
            }
        }

        /// The model to use: the chosen one, else the service's default, else its first free model.
        fn model(&self) -> String {
            if !self.settings.model.is_empty() {
                return self.settings.model.clone();
            }
            let Some(p) = self.provider() else { return String::new() };
            let d = default_model(&p);
            if !d.is_empty() {
                return d.to_string();
            }
            let list = self.models.get(&p.id).map(Vec::as_slice).unwrap_or_default();
            list.iter().find(|m| m.free).or(list.first()).map(|m| m.id.clone()).unwrap_or_default()
        }

        fn save(&mut self) {
            if let Some(d) = &self.dir
                && let Err(e) = self.settings.save(d)
            {
                self.notice = Some(format!("{} {e}", l("تعذّر حفظ الإعدادات:", "Couldn't save the settings:")));
            }
        }

        /// List the service's models in the background, once.
        fn load_models(&mut self, ctx: &egui::Context) {
            let Some(p) = self.provider() else { return };
            if self.models.contains_key(&p.id) || self.loading.is_some() {
                return;
            }
            let key = self.key(&p.id);
            if p.needs_key && key.is_empty() {
                return;
            }
            let (tx, rx) = channel();
            self.loading = Some((p.id.clone(), rx));
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new().name("agent-models".into()).spawn(move || {
                let _ = tx.send(list_models(&p, &key).map_err(|e| e.to_string()));
                ctx.request_repaint();
            });
            if spawned.is_err() {
                self.loading = None;
            }
        }
    }

    fn state(app: &mut WordApp) -> &mut State {
        app.agent.state.get_or_insert_with(State::new)
    }

    /// Answer the worker's tool calls in this window, and take in its news.
    pub(super) fn drain(app: &mut WordApp, ctx: &egui::Context) {
        let Some(st) = app.agent.state.as_mut() else { return };
        if let Some((id, rx)) = &st.loading
            && let Ok(r) = rx.try_recv()
        {
            let id = id.clone();
            st.loading = None;
            match r {
                Ok(mut list) => {
                    list.sort_by(|a, b| b.free.cmp(&a.free).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
                    st.models.insert(id, list);
                }
                Err(e) => {
                    st.models.insert(id, Vec::new());
                    st.notice = Some(friendly(&e));
                }
            }
        }
        let requests: Vec<ControlRequest> = st.req_rx.try_iter().collect();
        for req in requests {
            let reply = req.reply.clone();
            let v = match crate::control::handle(app, ctx, &req) {
                Outcome::Done(v) => v,
                Outcome::Screenshot { .. } => json!({"ok": false, "error": "screenshots are not available to the agent"}),
            };
            let _ = reply.send(v);
        }
        let Some(st) = app.agent.state.as_mut() else { return };
        let mut done: Option<Option<String>> = None;
        if let Some(run) = st.run.as_mut() {
            while let Ok(msg) = run.rx.try_recv() {
                match msg {
                    Msg::Event(RunEvent::Text(t)) => run.text.push_str(&t),
                    Msg::Event(RunEvent::Assistant(m)) => {
                        run.text.clear();
                        st.chat.push(m);
                    }
                    Msg::Event(RunEvent::ToolResults(m)) => st.chat.push(m),
                    Msg::Event(RunEvent::ToolStarted { id, .. }) => run.tool = Some(id),
                    Msg::Event(RunEvent::ToolFinished { .. }) => run.tool = None,
                    Msg::Event(RunEvent::Usage { cost, .. }) => st.cost += cost.unwrap_or(0.0),
                    // The agent's tools never ask; refuse anything that does.
                    Msg::Event(RunEvent::Approve { reply, .. }) => {
                        let _ = reply.send(wordcraft_agent::run::Approval::No);
                    }
                    Msg::Event(RunEvent::Thinking(_) | RunEvent::Finished(_)) => {}
                    Msg::Done(e) => done = Some(e),
                }
            }
        }
        if let Some(error) = done
            && let Some(run) = st.run.take()
        {
            st.notice = error;
            // The whole turn is one Undo step, while its document is still the active one.
            if app.session.document_id() == run.doc {
                app.session.squash_undo_since(run.mark, super::title());
            }
        }
    }

    /// An error as the person should read it.
    fn friendly(e: &str) -> String {
        let has = |s: &str| e.contains(s);
        if has(" 401") || has(" 403") || has("API key not valid") || has("invalid x-api-key") {
            l("المفتاح غير صحيح. افتح «الخدمة» وأدخل مفتاحًا جديدًا.", "The key isn't valid. Open Service and enter a new key.").to_string()
        } else if has(" 429") {
            l(
                "تجاوزت حد الاستخدام لهذا النموذج الآن. انتظر قليلًا أو اختر نموذجًا آخر.",
                "You've reached this model's usage limit for now. Wait a little or choose another model.",
            )
            .to_string()
        } else if has("network") {
            format!("{} {e}", l("تعذّر الاتصال بالخدمة:", "Couldn't reach the service:"))
        } else {
            e.to_string()
        }
    }

    /// The model's instructions: who it is, what it may do, and what the person is looking at.
    fn system_prompt(app: &WordApp) -> String {
        let title = app.session.documents().into_iter().find(|d| d.active).map(|d| d.title).unwrap_or_default();
        let selection = wordcraft_agent::error::clip(&app.session.selected_text(), 4000);
        let mut s = format!(
            "You are وكيل (Wakeel), the AI agent inside كلمة (Kalima), an Arabic-first word processor. \
             The person is looking at their open document \"{title}\", and you change it live with your tools: \
             every command you run happens in their window while they watch.\n\
             - Read before you change: get_text for the words, inspect_document for structure, styles and the selection.\n\
             - Find command ids and their params with list_commands (a query such as \"bold\", \"table\", \"heading\", \"align\", \"font\"), \
             then run them with execute, or several in order with batch.\n\
             - select_text selects text to format or replace; type_text types at the caret and replaces the selection. \
             Use paragraphs: true to type several paragraphs at once.\n\
             - Work in small steps the person can follow. Don't open, save, close or print files: the person does that.\n\
             - Everything you do in one turn is a single Undo step for the person.\n\
             - Reply in the person's language, briefly: say what you changed, not how. Use Markdown sparingly.\n"
        );
        if selection.trim().is_empty() {
            s.push_str("Nothing is selected now.");
        } else {
            s.push_str(&format!("The person has selected this text:\n<selection>\n{selection}\n</selection>"));
        }
        s
    }

    /// Start a turn with the person's `text`.
    pub(super) fn send(app: &mut WordApp, ctx: &egui::Context, text: String) -> Result<(), String> {
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err("nothing to send".into());
        }
        if !app.has_document() {
            return Err(l("افتح مستندًا أولًا.", "Open a document first.").into());
        }
        let system = system_prompt(app);
        let mark = app.session.undo_mark();
        let doc = app.session.document_id();
        let st = state(app);
        if st.run.is_some() {
            return Err(l("وكيل يعمل الآن.", "The agent is working.").into());
        }
        if !st.ready() {
            return Err(l("اختر خدمة وأدخل مفتاحها أولًا.", "Choose a service and enter its key first.").into());
        }
        let Some(provider) = st.provider() else { return Err("no service".into()) };
        let model = st.model();
        if model.is_empty() {
            return Err(l("اختر نموذجًا أولًا.", "Choose a model first.").into());
        }
        let price = st.models.get(&provider.id).and_then(|v| v.iter().find(|m| m.id == model)).and_then(|m| m.price);
        let cfg = RunConfig { key: st.key(&provider.id), provider, model, system, effort: st.settings.effort.clone(), price };
        st.chat.push(Message::user_text(text));
        st.notice = None;
        let history = st.chat.clone();
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut tools = AppTools { tx: st.req_tx.clone(), ctx: ctx.clone() };
        let (worker_cancel, worker_ctx) = (cancel.clone(), ctx.clone());
        let spawned = std::thread::Builder::new().name("agent".into()).spawn(move || {
            let mut allowed = std::collections::HashSet::new();
            let r = wordcraft_agent::run::run(&cfg, &mut tools, history, &mut allowed, &worker_cancel, &mut |e| {
                let _ = tx.send(Msg::Event(e));
                worker_ctx.request_repaint();
            });
            let error = match r {
                Ok(()) | Err(Error::Cancelled) => None,
                Err(e) => Some(friendly(&e.to_string())),
            };
            let _ = tx.send(Msg::Done(error));
            worker_ctx.request_repaint();
        });
        if let Err(e) = spawned {
            st.chat.pop();
            return Err(e.to_string());
        }
        st.run = Some(Running { rx, cancel, text: String::new(), tool: None, mark, doc });
        Ok(())
    }

    /// The tools, run in the window through the control channel.
    struct AppTools {
        tx: Sender<ControlRequest>,
        ctx: egui::Context,
    }

    impl wordcraft_mcp::Backend for AppTools {
        fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
            let (req, rx) = ControlRequest::new(method, params);
            self.tx.send(req).map_err(|_| "the window closed".to_string())?;
            self.ctx.request_repaint();
            let v = rx.recv_timeout(TOOL_TIMEOUT).map_err(|_| "the window did not answer".to_string())?;
            if v.get("ok").and_then(Value::as_bool) == Some(true) {
                Ok(v.get("result").cloned().unwrap_or(Value::Null))
            } else {
                Err(v.get("error").and_then(Value::as_str).unwrap_or("failed").to_string())
            }
        }
        fn has_ui(&self) -> bool {
            true
        }
        fn describe(&self) -> String {
            "the open window".into()
        }
    }

    /// Commands the agent may not run: files, the window and Help belong to the person.
    fn blocked(command: &str) -> bool {
        command.starts_with("file.") || command.starts_with("ui.") || command.starts_with("help.") || command == "view.switchWindows"
    }

    impl Tools for AppTools {
        fn specs(&self) -> Vec<ToolSpec> {
            let defs = wordcraft_mcp::tools::tool_definitions();
            defs.as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter_map(|d| {
                    let name = d.get("name")?.as_str()?;
                    TOOLS.contains(&name).then_some(())?;
                    let mut schema = d.get("inputSchema").cloned().unwrap_or(json!({"type": "object"}));
                    // One document: the one in this window.
                    if let Some(props) = schema.get_mut("properties").and_then(Value::as_object_mut) {
                        props.remove("doc");
                    }
                    Some(ToolSpec { name: name.into(), description: d.get("description")?.as_str()?.into(), schema })
                })
                .collect()
        }

        fn needs_approval(&self, _name: &str) -> bool {
            false
        }

        fn owner(&self, _name: &str) -> String {
            String::new()
        }

        fn call(&mut self, name: &str, input: &Value) -> wordcraft_agent::Result<String> {
            if !TOOLS.contains(&name) {
                return Err(Error::Tool(format!("unknown tool `{name}`")));
            }
            if input.get(INVALID_INPUT).is_some() {
                return Err(Error::Tool("the arguments were not valid JSON; send an object".into()));
            }
            let mut input = input.clone();
            if let Some(o) = input.as_object_mut() {
                o.remove("doc");
            }
            let commands: Vec<&str> = match name {
                "execute" => input.get("command").and_then(Value::as_str).into_iter().collect(),
                "batch" => input
                    .get("commands")
                    .and_then(Value::as_array)
                    .map(|v| v.iter().filter_map(|c| c.get("command").and_then(Value::as_str)).collect())
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            if let Some(c) = commands.into_iter().find(|c| blocked(c)) {
                return Err(Error::Tool(format!("`{c}` is for the person to run, not the agent")));
            }
            let r = wordcraft_mcp::tools::call_tool(self, name, &input);
            let text: Vec<&str> = r.content.iter().filter_map(|c| c.get("text").and_then(Value::as_str)).collect();
            let text = wordcraft_agent::error::clip(&text.join("\n"), MAX_RESULT);
            if r.is_error { Err(Error::Tool(text)) } else { Ok(text) }
        }
    }

    /// A tool call as one short line: the command's label where there is one.
    fn step_label(app: &WordApp, name: &str, input: &Value) -> String {
        let command = |id: &str| app.session.registry.get(id).map(|c| crate::i18n::t(c.label).to_string()).unwrap_or_else(|| id.to_string());
        match name {
            "execute" => command(input.get("command").and_then(Value::as_str).unwrap_or("")),
            "batch" => {
                let ids: Vec<String> = input
                    .get("commands")
                    .and_then(Value::as_array)
                    .map(|v| v.iter().filter_map(|c| c.get("command").and_then(Value::as_str)).take(4).map(command).collect())
                    .unwrap_or_default();
                ids.join(l("، ", ", "))
            }
            "type_text" => l("كتابة نص", "Typing text").into(),
            "select_text" => l("تحديد نص", "Selecting text").into(),
            "get_text" => l("قراءة المستند", "Reading the document").into(),
            "inspect_document" => l("فحص المستند", "Inspecting the document").into(),
            "list_commands" => l("البحث عن أمر", "Looking up commands").into(),
            other => other.into(),
        }
    }

    /// The suggestions shown on an empty chat: label and request.
    fn suggestions() -> [(&'static str, &'static str); 5] {
        [
            (
                l("لخّص المستند", "Summarize this document"),
                l("لخّص المستند في فقرة قصيرة في آخره.", "Summarize the document in a short paragraph at its end."),
            ),
            (
                l("حسّن النص المحدد", "Improve the selected text"),
                l("أعد صياغة النص المحدد ليكون أوضح وأسلس.", "Rewrite the selected text so it's clearer and smoother."),
            ),
            (
                l("صحّح الأخطاء", "Fix spelling and grammar"),
                l("صحّح الأخطاء الإملائية والنحوية في المستند.", "Fix the spelling and grammar mistakes in the document."),
            ),
            (
                l("نسّق كتقرير", "Format as a report"),
                l("نسّق المستند كتقرير: عنوان رئيسي وعناوين للأقسام.", "Format the document as a report: a title and section headings."),
            ),
            (l("أضف جدولًا", "Add a table"), l("أضف جدولًا مناسبًا للمحتوى في مكان المؤشر.", "Add a table that suits the content at the caret.")),
        ]
    }

    pub(super) fn pane(app: &mut WordApp, ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());
        let ctx = ui.ctx().clone();
        let ready = state(app).ready();
        if !ready || state(app).setup.is_some() {
            setup(app, ui, &t);
            return;
        }
        state(app).load_models(&ctx);
        header(app, ui);
        ui.separator();
        // The chat fills the pane above the input box.
        let input_height = 110.0;
        let chat_height = (ui.available_height() - input_height).max(80.0);
        let mut ask: Option<String> = None;
        egui::ScrollArea::vertical().id_salt("agent_chat").max_height(chat_height).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if state(app).chat.is_empty() && state(app).run.is_none() {
                ask = welcome(ui, &t);
            } else {
                messages(app, ui, &t);
            }
        });
        if let Some(n) = state(app).notice.clone() {
            ui.colored_label(ui.visuals().error_fg_color, n);
        }
        if let Some(text) = ask.or_else(|| input(app, ui)) {
            if let Err(e) = send(app, &ctx, text) {
                state(app).notice = Some(e);
            } else {
                state(app).input.clear();
            }
        }
    }

    fn header(app: &mut WordApp, ui: &mut Ui) {
        let st = state(app);
        let providers = st.settings.providers();
        let current = st.provider().map(|p| p.name).unwrap_or_default();
        let model = st.model();
        let models = st.provider().and_then(|p| st.models.get(&p.id).cloned());
        let mut pick_provider: Option<String> = None;
        let mut pick_model: Option<String> = None;
        let mut new_chat = false;
        let w = ui.available_width();
        ui.horizontal(|ui| {
            craft_ui::ComboBox::from_id_salt("agent_provider").width((w * 0.35).max(60.0)).selected_text(current).show_ui(ui, |ui| {
                for p in &providers {
                    if ui.selectable_label(false, &p.name).clicked() {
                        pick_provider = Some(p.id.clone());
                    }
                }
            });
            let label = models.as_ref().and_then(|v| v.iter().find(|m| m.id == model)).map(|m| m.name.clone()).unwrap_or_else(|| model.clone());
            craft_ui::ComboBox::from_id_salt("agent_model").width((w * 0.45).max(60.0)).selected_text(label).show_ui(ui, |ui| match &models {
                None => {
                    ui.label(l("جارٍ تحميل النماذج…", "Loading models…"));
                }
                Some(list) => {
                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        for m in list {
                            let name = if m.free { format!("{} · {}", m.name, l("مجاني", "free")) } else { m.name.clone() };
                            if ui.selectable_label(m.id == model, name).clicked() {
                                pick_model = Some(m.id.clone());
                            }
                        }
                    });
                }
            });
            if ui.button("+").on_hover_text(l("محادثة جديدة", "New chat")).clicked() {
                new_chat = true;
            }
        });
        if let Some(id) = pick_provider {
            st.settings.provider = id;
            st.settings.model.clear();
            st.notice = None;
            if !st.ready() {
                st.setup = Some(st.settings.provider.clone());
            }
            st.save();
        }
        if let Some(m) = pick_model {
            st.settings.model = m;
            st.save();
        }
        if new_chat && st.run.is_none() {
            st.chat.clear();
            st.notice = None;
            st.cost = 0.0;
        }
    }

    /// The empty chat: what the agent does, and requests to start from.
    fn welcome(ui: &mut Ui, t: &Tokens) -> Option<String> {
        ui.add_space(8.0);
        ui.label(RichText::new(l("مرحبًا، أنا وكيل.", "Hi, I'm your AI Agent.")).font(crate::theme::semibold(16.0)).color(t.text));
        ui.label(
            RichText::new(l(
                "اطلب مني ما تريد في هذا المستند، فأكتب وأنسّق وأعدّل أمامك. يمكنك التراجع عن كل ما أفعله بضغطة واحدة.",
                "Ask me for anything in this document and I'll write, format and edit it as you watch. One Undo takes back everything I did.",
            ))
            .color(t.text_dim),
        );
        ui.add_space(10.0);
        let mut ask = None;
        for (label, request) in suggestions() {
            let b = egui::Button::new(RichText::new(label).color(t.text)).min_size(egui::vec2(ui.available_width(), 28.0));
            if ui.add(b).clicked() {
                ask = Some(request.to_string());
            }
        }
        ask
    }

    fn messages(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
        let Some(st) = app.agent.state.as_ref() else { return };
        // Which tool calls failed, from their results.
        let mut failed: HashMap<&str, bool> = HashMap::new();
        for m in &st.chat {
            for p in &m.parts {
                if let Part::ToolResult { id, is_error, .. } = p {
                    failed.insert(id.as_str(), *is_error);
                }
            }
        }
        let running_tool = st.run.as_ref().and_then(|r| r.tool.clone());
        let copy = l("نسخ", "Copy");
        for m in &st.chat {
            match m.role {
                Role::User => {
                    let text = m.text();
                    if text.trim().is_empty() {
                        continue;
                    }
                    ui.add_space(6.0);
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        egui::Frame::new().fill(t.checked).corner_radius(10).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                            ui.set_max_width(ui.available_width() * 0.85);
                            ui.label(RichText::new(text).color(t.text));
                        });
                    });
                }
                Role::Assistant => {
                    for p in &m.parts {
                        match p {
                            Part::Text { text } if !text.trim().is_empty() => {
                                ui.add_space(4.0);
                                crate::agent_markdown::show(ui, text, t, copy);
                            }
                            Part::ToolCall { id, name, input } => {
                                let (mark, color) = match failed.get(id.as_str()) {
                                    Some(false) => ("✓", t.accent_text),
                                    Some(true) => ("✗", ui.visuals().error_fg_color),
                                    None if running_tool.as_deref() == Some(id) => ("…", t.text_dim),
                                    None => ("·", t.text_dim),
                                };
                                let label = step_label(app, name, input);
                                ui.label(RichText::new(format!("{mark} {label}")).small().color(color));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if let Some(run) = &st.run {
            ui.add_space(4.0);
            if run.text.is_empty() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(l("يعمل…", "Working…")).color(t.text_dim));
                });
            } else {
                crate::agent_markdown::show(ui, &run.text, t, copy);
            }
        }
        if st.cost > 0.0 {
            ui.add_space(6.0);
            ui.label(RichText::new(format!("${:.4}", st.cost)).small().color(t.text_dim));
        }
    }

    /// The message box and Send/Stop. Returns the text to send.
    fn input(app: &mut WordApp, ui: &mut Ui) -> Option<String> {
        let st = state(app);
        let running = st.run.is_some();
        let w = ui.available_width();
        let edit = egui::TextEdit::multiline(&mut st.input)
            .id_salt("agent_input")
            .hint_text(l("اطلب من وكيل…", "Ask the AI Agent…"))
            .desired_rows(3)
            .desired_width(w)
            // Enter sends; Shift+Enter starts a new line.
            .return_key(Some(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)));
        let r = ui.add(edit);
        let enter = r.has_focus() && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
        let mut send = None;
        ui.horizontal(|ui| {
            if running {
                if ui.button(l("إيقاف", "Stop")).clicked()
                    && let Some(run) = &st.run
                {
                    run.cancel.store(true, Ordering::Relaxed);
                }
            } else if (ui.add_enabled(!st.input.trim().is_empty(), egui::Button::new(l("إرسال", "Send"))).clicked() || enter)
                && !st.input.trim().is_empty()
            {
                send = Some(st.input.clone());
            }
            if ui.small_button(l("الخدمة", "Service")).clicked() {
                st.setup = Some(st.settings.provider.clone());
                st.key_input.clear();
            }
        });
        send
    }

    /// Choosing a service and entering its key.
    fn setup(app: &mut WordApp, ui: &mut Ui, t: &Tokens) {
        let st = state(app);
        let providers = st.settings.providers();
        let mut chosen = st
            .setup
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| if st.settings.provider.is_empty() { "gemini".into() } else { st.settings.provider.clone() });
        ui.add_space(6.0);
        ui.label(RichText::new(l("اختر خدمة الذكاء الاصطناعي", "Choose an AI service")).font(crate::theme::semibold(15.0)).color(t.text));
        ui.label(
            RichText::new(l(
                "‏Gemini مجاني: أنشئ مفتاحًا من Google AI Studio والصقه هنا. ويقدّم OpenRouter نماذج مجانية أيضًا.",
                "Gemini is free: create a key in Google AI Studio and paste it here. OpenRouter has free models too.",
            ))
            .color(t.text_dim),
        );
        ui.add_space(6.0);
        for id in SETUP {
            let Some(p) = providers.iter().find(|p| p.id == *id) else { continue };
            let name = match *id {
                "gemini" => format!("{} ({})", p.name, l("مجاني", "free")),
                "openrouter" => format!("{} ({})", p.name, l("فيه نماذج مجانية", "has free models")),
                "ollama" => format!("{} ({})", p.name, l("على جهازك", "on this computer")),
                _ => p.name.clone(),
            };
            if ui.radio(chosen == p.id, name).clicked() {
                chosen = p.id.clone();
                st.key_input.clear();
            }
        }
        st.setup = Some(chosen.clone());
        let Some(p) = providers.iter().find(|p| p.id == chosen).cloned() else { return };
        ui.add_space(6.0);
        let has_key = !st.key(&p.id).is_empty();
        if p.needs_key {
            ui.horizontal(|ui| {
                ui.label(l("المفتاح:", "Key:"));
                ui.hyperlink_to(l("احصل على مفتاح", "Get a key"), &p.key_url);
            });
            let hint = if has_key {
                l("محفوظ. الصق مفتاحًا جديدًا لتغييره.", "Saved. Paste a new key to change it.")
            } else {
                l("الصق المفتاح هنا", "Paste the key here")
            };
            ui.add(egui::TextEdit::singleline(&mut st.key_input).password(true).hint_text(hint).desired_width(ui.available_width()));
        } else {
            ui.label(
                RichText::new(l("شغّل Ollama على جهازك أولًا، ثم اضغط «ابدأ».", "Start Ollama on this computer first, then press Start."))
                    .color(t.text_dim),
            );
        }
        ui.add_space(6.0);
        let can = !p.needs_key || has_key || !st.key_input.trim().is_empty();
        let mut start = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            start = ui
                .add_enabled(
                    can,
                    egui::Button::new(RichText::new(l("ابدأ", "Start")).color(if can { t.on_accent } else { t.text_disabled })).fill(if can {
                        t.accent
                    } else {
                        Color32::TRANSPARENT
                    }),
                )
                .clicked();
            if st.ready() && ui.button(l("إلغاء", "Cancel")).clicked() {
                cancel = true;
            }
        });
        if cancel {
            st.setup = None;
            st.key_input.clear();
        }
        if start {
            let key = st.key_input.trim().to_string();
            if !key.is_empty() {
                let saved = st.dir.as_deref().map(|d| secrets::set(d, &p.id, &key));
                match saved {
                    Some(Ok(())) => {
                        st.keys.insert(p.id.clone(), key);
                    }
                    Some(Err(e)) => {
                        st.notice = Some(format!("{} {e}", l("تعذّر حفظ المفتاح:", "Couldn't save the key:")));
                        return;
                    }
                    None => {
                        // No settings folder: keep the key for this run only.
                        st.keys.insert(p.id.clone(), key);
                    }
                }
            }
            if st.settings.provider != p.id {
                st.settings.model.clear();
            }
            st.settings.provider = p.id.clone();
            st.models.remove(&p.id);
            st.setup = None;
            st.key_input.clear();
            st.notice = None;
            st.save();
        }
        if let Some(n) = &st.notice {
            ui.colored_label(ui.visuals().error_fg_color, n);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(l(
                "يُحفظ مفتاحك في سلسلة مفاتيح جهازك ولا يُرسل إلا إلى الخدمة التي اخترتها.",
                "Your key is kept in this computer's keychain and only ever sent to the service you chose.",
            ))
            .small()
            .color(t.text_dim),
        );
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_agent_cannot_touch_files_or_the_window() {
            for c in ["file.save", "file.open", "file.close", "ui.dialog", "help.about", "view.switchWindows"] {
                assert!(blocked(c), "{c}");
            }
            for c in ["format.bold", "insert.table", "text.insert", "select.all"] {
                assert!(!blocked(c), "{c}");
            }
        }

        #[test]
        fn the_agent_sees_only_its_tools_and_one_document() {
            let (tx, _rx) = channel();
            let tools = AppTools { tx, ctx: egui::Context::default() };
            let specs = tools.specs();
            let names: Vec<&str> = specs.iter().map(|s| s.name.as_str()).collect();
            assert_eq!(names.len(), TOOLS.len(), "{names:?}");
            for s in &specs {
                assert!(s.schema.get("properties").and_then(|p| p.get("doc")).is_none(), "{} still offers `doc`", s.name);
            }
        }

        #[test]
        fn blocked_commands_never_reach_the_window() {
            let (tx, rx) = channel();
            let mut tools = AppTools { tx, ctx: egui::Context::default() };
            let e = tools.call("execute", &json!({"command": "file.save"}));
            assert!(e.is_err());
            let e = tools.call("batch", &json!({"commands": [{"command": "format.bold"}, {"command": "file.close"}]}));
            assert!(e.is_err());
            let e = tools.call("screenshot", &json!({}));
            assert!(e.is_err());
            assert!(rx.try_recv().is_err(), "nothing was sent to the window");
        }

        #[test]
        fn errors_read_plainly() {
            assert!(friendly("Gemini returned 401: bad").contains(l("المفتاح", "key")));
            assert!(friendly("OpenRouter returned 429: slow down").contains(l("حد", "limit")));
        }
    }
}
