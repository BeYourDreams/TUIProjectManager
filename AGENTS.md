# TUIProjectManager 开发规则

## UI 输入/焦点（高频回归点）
- 终端在 `term_focused` 时会每帧抢回键盘焦点（见 `src/terminal.rs` `show_terminal` 焦点锁）。**任何覆盖终端且含文本输入的 UI（设置浮层、输入弹窗等）显示时，必须抑制终端焦点**，否则输入框表现为“光标闪烁无法输入”。
- 参考实现：`src/app.rs` 会话渲染时传 `term_focused && !settings_open`，打开期间不回写终端焦点。新增类似覆盖层时必须同步处理。
- 详见 `docs/ui-input-focus.md`。
