
set rl [open "|target/debug/rl" w+]

proc key { chr } {
    global rl
    puts $rl "key $chr down"
    puts $rl "key $chr up"
    flush $rl
}

proc up { } { key 8 }
proc down { } { key 2 }
proc left { } { key 4 }
proc right { } { key 6 }
proc upleft { } { key 7 }
proc upright { } { key 9 }
proc downleft { } { key 1 }
proc downright { } { key 3 }
proc pass {} { key 5 }


