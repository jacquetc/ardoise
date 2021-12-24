#!/bin/sh
xrandr --newmode "1408x1872_30.00"  106.00  1408 1488 1632 1856  1872 1875 1885 1907 -hsync +vsync
xrandr --addmode HDMI-1 1408x1872_30.00
xrandr -s 1408x1872
ardoise -r90 > /var/log/ardoise.log 2>&1 & 
