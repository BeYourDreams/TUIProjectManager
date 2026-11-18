# UI 输入焦点规则（防回归）

## 现象
设置页/弹层中的输入框“光标闪烁但无法输入”，通常是键盘焦点被每帧抢走。

## 规则
1. 终端焦点锁（`src/terminal.rs` `show_terminal`）：当 `*term_focused == true` 时会在每帧把焦点拽回终端并独占 Tab/方向键/Esc。这是为避免终端内操作把焦点落到页签栏导致误切页。
2. **凡是出现可键盘输入的覆盖层/前景 UI（设置浮层、含 TextEdit 的对话框等），渲染终端时必须屏蔽终端焦点**：向 `show_terminal` 传入的生效值应为 `term_focused && !overlay_open`，且覆盖层打开期间不要把该值回写为“已聚焦终端”。否则输入框会出现“光标闪烁无法输入”。
3. 新增任何会覆盖终端且包含输入控件的 UI 时，必须检查：终端是否仍在渲染、`term_focused` 是否被抑制、焦点是否会被每帧夺回。

## 相关代码
- `src/terminal.rs`: 终端焦点锁与 `request_focus`
- `src/app.rs`: 会话页签渲染处对 `settings_open` 的焦点屏蔽
