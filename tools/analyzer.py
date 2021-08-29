import time
from math import cos, sin
import dearpygui.dearpygui as dpg


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
    counts = {}

    for line in lines:
        if not "Elapsed" in line:
            continue

        parts = line.split()
        name = parts[3][:-1]
        if not name in series.keys():
            series[name] = [(0.0, 0)]
            totals[name] = 0
            counts[name] = 0

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
        counts[name] += 1

        last_time = max(last_time, timestamp)

        names.add(name)

    names = list(names)
    names.sort()
    names.reverse()

    total_time = sum(totals.values())
    for name in names:
        series[name].append((last_time, 0))
        avg_us = totals[name] / float(counts[name])
        percent = totals[name] / total_time
        print("{0:10}: {1:6}, {2:.6f}, {3:.6f} {4:.2f}".format(name, counts[name], totals[name], avg_us, percent))
    print()
    
    return (names, series)

def load_perf(file_name):
    with open(file_name, 'r') as fh:
        lines = fh.readlines()

    (names, series) = parse_series(lines)

    return (names, series)

def plot_perf(plot_axis):
    (names, series) = load_perf("game.log")

    index = 1 + 3 * len(names)
    for (name, data) in series.items():
        xs = [pair[0] for pair in data]
        ys = [pair[1] + (index / 2) for pair in data]
        dpg.add_line_series(xs, ys, label=name, parent=plot_axis) #, weight=2, fill=[255, 0, 0, 100])
        index -= 3

def plot_callback(sender, data):
    plot_perf(plot_axis)

dpg.setup_viewport()
width = dpg.get_viewport_width()
height = dpg.get_viewport_height()

with dpg.window(label="RRL Performance", width=width, height=height, no_move=True, no_collapse=True, no_title_bar=True) as window:

    dpg.add_text("Performance Plot")
    dpg.add_button(label="Reload file", callback=plot_callback)

    with dpg.plot(label="Perf", width=width - 40, height=height - 120):
        dpg.add_plot_legend()
        dpg.add_plot_axis(dpg.mvXAxis, label='x')
        plot_axis = dpg.add_plot_axis(dpg.mvYAxis, label='y')

    plot_perf(plot_axis)

dpg.start_dearpygui()
