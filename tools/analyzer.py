import time
from math import cos, sin
from dearpygui.dearpygui import *


class Frame:
    def __init__(self, start, end, data):
        self.start = start
        self.end = end
        self.data = data

def parse_timestamp(timestamp):
    time_parts = timestamp.split(":")
    return (float(time_parts[0]) * 3600.0) + (float(time_parts[1]) * 60.0) + float(time_parts[2])

def parse_series(lines):
    """
    Given lines of the form
        [HH:MM:SS.SSS] (ID) LOGLEVEL TAG, Elapsed=XX.YYms

    Extrace the TAG values as 'names', and for each tag a list of (timestamp, duration) pairs
    where the timestamp is at the end of the section that is being monitored
    """
    series = {}
    skip_n = len("Elapsed=")
    names = set()
    epsilon = 0.0000000001
    last_time = 0.0
    totals = {}

    for line in lines:
        if not "Elapsed" in line:
            continue

        parts = line.split()
        name = parts[3][:-1]
        if not name in series.keys():
            series[name] = [(0.0, 0)]
            totals[name] = 0

        elapsed_str = parts[4][skip_n:-3]
        timestamp = parse_timestamp(parts[0][1:-1])
        
        if elapsed_str == '':
            continue

        elapsed = float(elapsed_str)
        stripped_line = line.strip()
        if stripped_line.endswith('ms'):
            elapsed /= 1000.0
        elif not stripped_line[-2].isdigit():
            elapsed /= 1000000.0

        series[name].append((timestamp - elapsed, 0))
        series[name].append((timestamp - elapsed + epsilon, 1))
        series[name].append((timestamp, 1))
        series[name].append((timestamp + epsilon, 0))

        totals[name] += elapsed

        last_time = max(last_time, timestamp)

        names.add(name)

    names = list(names)
    names.sort()
    names.reverse()

    for name in names:
        series[name].append((last_time, 0))
        print(name + " (" + str(len(series[name])) + "): " + str(totals[name]))
    print()
    
    return (names, series)

def load_perf(file_name):
    with open(file_name, 'r') as fh:
        lines = fh.readlines()

    (names, series) = parse_series(lines)

    return (names, series)

def plot_perf():
    (names, series) = load_perf("game.log")

    index = 1 + 3 * len(names)
    for (name, data) in series.items():
        add_line_series("Plot", name, [(pair[0], pair[1] + (index / 2)) for pair in data]) #, weight=2, fill=[255, 0, 0, 100])
        index -= 3

def plot_callback(sender, data):
    clear_plot("Plot")
    plot_perf()

add_text("Performance Plot")
add_button("Reload file", callback=plot_callback)
add_plot("Plot", "x-axis", "y-axis", height=-1)

clear_plot("Plot")
plot_perf()

start_dearpygui()
