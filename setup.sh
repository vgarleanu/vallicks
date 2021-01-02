#!/bin/bash

sudo ip link add br0 type bridge
sudo ip addr flush dev br0
sudo ip addr add 192.168.100.50/24 brd 192.168.100.255 dev br0
sudo ip tuntap add mode tap user hinach4n
ip tuntap show
sudo ip link set tap0 master br0
sudo ip link set dev br0 up
sudo ip link set dev tap0 up
sudo dnsmasq --interface=br0 --bind-interfaces --dhcp-range=192.168.100.50,192.168.100.254
