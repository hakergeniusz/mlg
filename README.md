# mlg

A Horizon native mod: places your water bucket under you in the last stretch
of a fall, so no fall damage registers.

Fall damage in Horizon comes from the world that runs the physics — on a
server, that is the server's own simulation of your body. This mod does not
touch any number: it makes a real, legal use of a real water bucket at the
block above the floor your feet will reach, inside the server's own reach
check. The server's simulation sees water before you land, and the fall
height it charges resets — on any server, with or without mods.

It picks the water back up once you have landed, so the bucket comes home.

Needs a water bucket in the hotbar. Latency is the honest limit: the water
must exist before your feet arrive, and a use only leads your fall by a few
ticks — a LAN or a nearby dedicated server saves most falls; a distant
server saves the slower ones.

MIT. Clean-room: written for Horizon from the game's own public behavior.
