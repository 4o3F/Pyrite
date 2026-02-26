# Pyrite
A fast, memory saving ICPC contest resolver for DomJudge.

## Uage
The program require a CDP (Contest Data Path) to run, example structure is as follow.
```
.
├── affiliations
│   ├── INST-001.jpg
│   ├── ....
│   └── INST-235.jpg
├── config.toml
├── event-feed.ndjson
└── teams
    ├── team002.jpg
    ├── ....
    └── team417.jpg
```
use GUI to set CDP path, let the program auto validate and parse the event feeds. 

> [!NOTE]  
> The config file is optional, which can be used to define several behavior and some post processing of event feed (to fix broken event feed, use issue for new post processing feature request).


## Build
For total static AOT build, place following static libraries inside `Native` to link.
+ https://github.com/2ndlab/ANGLE.Static
+ https://github.com/2ndlab/SkiaSharp.Static
Then it should be fully self contained.
