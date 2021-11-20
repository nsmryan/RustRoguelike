
proc start { args } {
    global rl

    if { [info exists rl] } {
        if { $rl != "" } {
            catch quit
        }
    }
    set rl ""

    #set exe rl
    set exe rl_engine
    set command [join [list "|target/debug/$exe.exe" {*}$args] " "]
    puts $command
    set rl [open  $command w+]
    fconfigure $rl -buffering line -blocking 0
}

proc is_prefix { prefix str } {
    return [string equal -length [string length $prefix] $prefix $str]
}

proc cmd { name args } {
    global rl

    set command $name
    lappend command {*}$args
    puts $rl $command

    set line [gets $rl]
    while { ![is_prefix "OUTPUT: $name" $line] } {
        if { [string length $line] > 0 } {
            puts $line
        }
        set line [gets $rl]
    }

    return [lrange $line 2 end]
}

proc make_cmd { name } {
    uplevel 1 "proc $name { args } { cmd $name {*}\$args }"
}

proc press_key { chr } {
    cmd key $chr down
    cmd key $chr up
    #global rl
    #puts $rl "key $chr down"
    #puts $rl "key $chr up"
    #flush $rl
}

proc up { } { press_key 8 }
proc down { } { press_key 2 }
proc left { } { press_key 4 }
proc right { } { press_key 6 }
proc upleft { } { press_key 7 }
proc upright { } { press_key 9 }
proc downleft { } { press_key 1 }
proc downright { } { press_key 3 }
proc pass {} { press_key 5 }
proc z {} { press_key z }
proc x {} { press_key x }
proc c {} { press_key c }
proc a {} { press_key a }
proc s {} { press_key s }
proc d {} { press_key d }
make_cmd give
make_cmd player_id
make_cmd hp
make_cmd set_pos
make_cmd pos
make_cmd facing
make_cmd map_size
make_cmd set_tile_walls
make_cmd surface
make_cmd set_surface
make_cmd entity_name
make_cmd entity_type
make_cmd spawn
make_cmd remove
make_cmd kill
make_cmd ids
make_cmd key
make_cmd ctrl
make_cmd alt
make_cmd shift

proc quit { } {
    global rl
    puts $rl exit
    flush $rl
    close $rl
    unset rl
}

#start_game

