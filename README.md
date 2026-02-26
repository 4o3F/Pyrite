# Pyrite
A fast, memory saving ICPC contest resolver for DomJudge.
Designed after being tortured by the ICPC Tool.
> [!CAUTION]
> This program is still experimental, use in production at your own risk

## Usage
Download the binary from release first.  It should be self contained and click to run.  
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

> [!TIP]  
> The config file is optional, which can be used to define several behavior and some post processing of event feed (to fix broken event feed, use issue for new post processing feature request).

Then set the awards inside GUI, `Gold`, `Silver`, `Bronze` medal winning team will be visualized, check before proceeding, there's no way back if presentation is launched.

> [!NOTE]  
> The category selection will also be used in resolver presentation, so remember to uncheck groups like `Star`

When resolve presentation is launched, pressing `F12` can toggle full screen, animation speed is configured inside config files and do not support on flight changes, so remember to do tests before presentation. Pressing `Space` will advance the resolve process.

## Build
For total static AOT build, place following static libraries inside `Native` to link.
+ https://github.com/2ndlab/ANGLE.Static
+ https://github.com/2ndlab/SkiaSharp.Static
Then it should be fully self contained.
