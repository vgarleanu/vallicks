#!/bin/bash

ip link add br0 type bridge
ip addr flush dev br0
ip addr add 192.168.100.50/24 brd 192.168.100.255 dev br0
ip tuntap add mode tap user hinach4n
ip tuntap show
ip link set tap0 master br0
ip link set dev br0 up
ip link set dev tap0 up
dnsmasq --interface=br0 --bind-interfaces --dhcp-range=192.168.100.50,192.168.100.254
