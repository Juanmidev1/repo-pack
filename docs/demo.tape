# Regenerate with: cargo build --release && vhs docs/demo.tape
Output docs/demo.gif

Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 820
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

Type "clear && repo-pack . --skeleton --stats"
Sleep 400ms
Enter
Sleep 4s

Type "clear && repo-pack . --preset arch > /dev/null"
Sleep 400ms
Enter
Sleep 5s
