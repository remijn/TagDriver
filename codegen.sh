#! /bin/sh

# zbus-xmlgen --session org.mpris.MediaPlayer2.playerctld /org/mpris/MediaPlayer2 >> ./src/zbus/playerctld.rs

dbus-codegen-rust -c nonblock -d org.mpris.MediaPlayer2.playerctld -p "/org/mpris/MediaPlayer2" -o ./src/dbus_interface/playerctld.rs
