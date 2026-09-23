Output docs/demo.gif

Set Shell "bash"
Set FontSize 19
Set Width 1200
Set Height 560
Set Padding 20
Set Framerate 24
Set PlaybackSpeed 1
Set TypingSpeed 12ms
Set Theme { "name": "Catppuccin Mocha", "black": "#45475a", "red": "#f38ba8", "green": "#a6e3a1", "yellow": "#f9e2af", "blue": "#89b4fa", "magenta": "#cba6f7", "cyan": "#94e2d5", "white": "#bac2de", "brightBlack": "#585b70", "brightRed": "#f38ba8", "brightGreen": "#a6e3a1", "brightYellow": "#f9e2af", "brightBlue": "#89b4fa", "brightMagenta": "#cba6f7", "brightCyan": "#94e2d5", "brightWhite": "#cdd6f4", "background": "#1e1e2e", "foreground": "#cdd6f4", "selection": "#585b70", "cursor": "#f5e0dc" }

Hide
Type "export PS1='$ '"
Enter
Type "memo --store demo init >/dev/null"
Enter
Type "clear"
Enter
Sleep 1s
Show
Sleep 500ms
Type "# Four notes stay in the raw log"
Enter
Sleep 500ms
Type "memo --store demo note 'Use one shared request ID in API logs and traces'"
Enter
Sleep 500ms
Type "memo --store demo note 'Include that request ID in every downstream call'"
Enter
Sleep 500ms
Sleep 1s
Type "memo --store demo nap 0-1 'Use one request ID across logs, traces, and downstream calls'"
Enter
Sleep 500ms
Type "memo --store demo note 'Run the focused checkout test before the full suite'"
Enter
Sleep 500ms
Type "memo --store demo note 'Keep the regression fixture scoped to checkout behavior'"
Enter
Sleep 500ms
Sleep 1s
Type "memo --store demo nap 2-3 'Run focused checkout tests with a scoped regression fixture first'"
Enter
Sleep 500ms
Sleep 1s

Hide
Type "clear"
Enter
Show
Type "# Pair summaries become the sources for one root"
Enter
Type "memo --store demo nap"
Enter
Sleep 2s
Type "memo --store demo nap 0-3 'Keep request IDs consistent and run focused checkout tests first'"
Enter
Sleep 1s

Hide
Type "clear"
Enter
Show
Type "# The line budget chooses root, pair summaries, or original notes"
Enter
Type "memo --store demo wake --lines 1"
Enter
Sleep 2s
Type "memo --store demo wake --lines 2"
Enter
Sleep 2s
Type "memo --store demo wake --lines 4"
Enter
Sleep 6s
