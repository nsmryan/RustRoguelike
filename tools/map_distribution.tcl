package require Tk
package require Plotchart


set current_script [info script]

proc cleanup {} {
    global updating

    foreach w [winfo children .] {
       destroy $w
    }

    after cancel $updating
}

proc reload {} {
   global current_script
   cleanup
   source $current_script
}


set filename "map_emptiness_distribution.txt" 

proc read_dist {} {
    global filename plot hist

    set fd [open $filename r]
    set data [read $fd]
    set lines [split $data "\n"]
    set points [list]
    foreach line $lines {
        lappend points [split $line " "]
    }

    return $points
}

proc plot_dist { plot hist points } {
    set max_count 0
    set max_amount 0
    foreach point $points {
        if {$point eq ""} {
            break
        }

        lassign $point x y
        set max_count [tcl::mathfunc::max $x $max_count]
        set max_amount [tcl::mathfunc::max $y $max_amount]
    }

    $plot deletedata
    $hist deletedata

    #$plot xconfig -scale "0 $max_count 10"
    #$plot yconfig -scale "0 $max_amount 10"

    #$hist xconfig -scale "0 $max_count 10"
    #$hist yconfig -scale "0 $max_amount 10"

    foreach point $points {
        if {$point eq ""} {
            break
        }

        lassign $point x y
        $plot plot dist $x $y
        $hist plot dist $x $y
    }
}

proc themeplot { plot } {
    Plotchart::plotconfig $plot leftaxis color white
    Plotchart::plotconfig $plot leftaxis textcolor white
    Plotchart::plotconfig $plot leftaxis thickness 2
    Plotchart::plotconfig $plot bottomaxis color white
    Plotchart::plotconfig $plot bottomaxis textcolor white
    Plotchart::plotconfig $plot bottomaxis thickness 2
}

themeplot xyplot
themeplot histogram

set width 400
set height 500
canvas .line -width $width -height $height
set plot [Plotchart::createXYPlot .line {0 50 10} {0 50 10}]
pack .line -side top -fill x
$plot dataconfig dist -colour white
$plot background axes black
$plot background plot grey

canvas .hist -width $width -height $height
set hist [Plotchart::createHistogram .hist {0 50 10} {0 50 10}]
pack .hist -side bottom -fill x
$hist background axes black
$hist background plot grey
$hist dataconfig dist -colour white -style filled -fillcolour white


proc update_plot {} {
    global plot hist updating

    plot_dist $plot $hist [read_dist]

    set updating [after 1000 update_plot]
}

update_plot
set updating [after 1000 update_plot]
vwait forever

