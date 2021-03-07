

set rl [open "|target/debug/roguelike_main" w+]

puts $rl "player_id"

while { set line [gets $rl] } {
    puts $line
}

exit

