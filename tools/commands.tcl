

proc start_game { args } {
    global rl

    if { [info exists rl] } {
        if { $rl != "" } {
            catch quit
        }
    }
    set rl ""

    set command [join [list "|target/debug/rl" {*}$args] " "]
    puts $command
    set rl [open  $command w+]
    fconfigure $rl -buffering line
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
    while { ![is_prefix "OUTPUT:" $line] } {
        set line [gets $rl]
    }

    return [lrange $line 2 end]
}

proc make_cmd { name } {
    uplevel 1 "proc $name { args } { cmd $name {*}\$args }"
}

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
proc z {} { key z }
proc x {} { key x }
proc c {} { key c }
proc a {} { key a }
proc s {} { key s }
proc d {} { key d }
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

