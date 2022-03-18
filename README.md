# Platformer test
![Screenshot](Screenshot.png)
A platformer template made in Rust, using `ggez` as framework and `tiled` for loading levels.

May evolve into an engine at some point; This repo is going to serve as the base for one of my future projects.

## Current / TODO Mechanics
### Level
- [x] Load a single level
- [x] Display multiple tile layers
- [ ] Camera movement within a single room
- [ ] Load multiple levels (rooms) from a Tiled world
- [ ] Minimap / Display map within pause menu

### Controls
- [x] Basic player movement
- [x] Simple AABB collision
- [x] Multijump
- [x] Sliding down walls
- [x] Walljump
- [ ] Sticking to walls
- [ ] Gravity switching

### Graphics
- [ ] Sprite atlas support
- [ ] Tile animation
- [ ] Player animation (Walking, jumping, etc)
- [ ] Support for externally defined (non-hardcoded) animation data

### Enemies
- [ ] Basic enemy AI (Walking left-right until collision)
- [ ] Player/enemy health & damage system
- [ ] Basic proyectile weapons

### Extra
- [ ] Pause menu
- [ ] Scripting support
- [ ] Cutscene utils
