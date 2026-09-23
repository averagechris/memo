Output docs/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1400
Set Height 820
Set Padding 24
Set Framerate 24
Set PlaybackSpeed 1
Set TypingSpeed 20ms
Set Theme { "name": "Catppuccin Mocha", "black": "#45475a", "red": "#f38ba8", "green": "#a6e3a1", "yellow": "#f9e2af", "blue": "#89b4fa", "magenta": "#cba6f7", "cyan": "#94e2d5", "white": "#bac2de", "brightBlack": "#585b70", "brightRed": "#f38ba8", "brightGreen": "#a6e3a1", "brightYellow": "#f9e2af", "brightBlue": "#89b4fa", "brightMagenta": "#cba6f7", "brightCyan": "#94e2d5", "brightWhite": "#cdd6f4", "background": "#1e1e2e", "foreground": "#cdd6f4", "selection": "#585b70", "cursor": "#f5e0dc" }

Hide
Type "export PS1='$ '"
Enter
Type "clear"
Enter
Show
Type "# Save two facts from today's work"
Sleep 1s
Enter
Type "memo --store demo init"
Enter
Sleep 1s
Type "memo --store demo note 'Use one shared request ID in API logs and traces'"
Enter
Sleep 1s
Type "memo --store demo note 'Run the focused checkout test before the full suite'"
Enter
Sleep 2s

Hide
Type "clear"
Enter
Show
Type "# A one-line wake needs an agent-supplied summary"
Sleep 1s
Enter
Type "memo --store demo wake --lines 1"
Enter
Sleep 5s

Hide
Type "clear"
Enter
Show
Type "# The agent groups both sources into one faithful line"
Sleep 1s
Enter
Type "memo --store demo nap 0-1 'Keep request IDs consistent and run focused tests first'"
Enter
Sleep 2s
Type "memo --store demo wake --lines 1"
Enter
Sleep 4s

Hide
Type "clear"
Enter
Show
Type "# Summaries save context. The original notes remain."
Sleep 1s
Enter
Type "memo --store demo wake --lines 2"
Enter
Sleep 5s
