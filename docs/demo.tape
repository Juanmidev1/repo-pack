# Regenerate with: cargo build --release && vhs docs/demo.tape
Output docs/demo.gif

Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 620
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 45ms

Hide
Type "export PATH=$PWD/target/release:$PATH PS1='$ ' && clear"
Enter
Show

Type "repo-pack . --stats"
Sleep 400ms
Enter
Sleep 4s

Type "clear && repo-pack . -e rs --max-tokens 100000 --copy"
Sleep 400ms
Enter
Sleep 3s

Type "clear && repo-pack . -e toml 2>/dev/null | head -20"
Sleep 400ms
Enter
Sleep 4s
