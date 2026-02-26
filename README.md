# Pyrite
A fast, memory-efficient ICPC contest resolver for DomJudge.
Created after being thoroughly tortured by the ICPC Tool.
> [!CAUTION]
> This program is still experimental. Use it in production at your own risk.

## Usage
First, download the binary from the Releases page. It is fully self-contained and can be run directly.  
The program requires a **CDP (Contest Data Path)** to operate. An example directory structure is shown below:
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
Use the GUI to set the CDP path. The program will automatically validate the structure and parse the event feed.

> [!TIP]  
> The `config.toml` file is optional. It can be used to customize behavior and apply post-processing to the event feed (for example, to fix malformed data). If you need additional post-processing features, please open an issue.

Next, configure the awards in the GUI. The `Gold`, `Silver`, and `Bronze` medal-winning teams will be visualized for review. Make sure to double-check everything before proceeding, once the presentation starts, it cannot be undone.

> [!NOTE]  
> The selected categories will also be used during the resolver presentation. Be sure to uncheck groups such as `Star` if they should not be included.

When the resolver presentation is running:

* Press `F12` to toggle full screen.
* Press `Space` to advance the resolution process.

Animation speed is configured in the `config.toml` file and cannot be changed during the presentation. Be sure to test everything beforehand.

Main logic is shown as following state machine.

![presentation flow](Docs/presentation_workflow.svg)

## Build
For a fully static AOT build, place the following static libraries inside the `Native` directory for linking:

* [https://github.com/2ndlab/ANGLE.Static](https://github.com/2ndlab/ANGLE.Static)
* [https://github.com/2ndlab/SkiaSharp.Static](https://github.com/2ndlab/SkiaSharp.Static)

After that, the build should be completely self-contained.
