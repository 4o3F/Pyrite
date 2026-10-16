# Pyrite
A fast, memory-efficient ICPC contest resolver for DomJudge.
Created after being thoroughly tortured by the ICPC Tool.
> [!IMPORTANT]
> This program is still experimental. Use it in production at your own risk.  
> Has been tested on **ICPC 2026 Xi'an Invitation Contest**.

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

On Launch, the presentation first shows the untouched frozen scoreboard at the top, pauses there for one second, then automatically scrolls to the bottom. No submissions are resolved during this opening browse, and advancement input is ignored until the browse finishes. `intro_scroll_seconds` controls the top-to-bottom scroll duration (default: 8 seconds, range: 0.1–3600 seconds), not including the initial one-second pause.

When the resolver presentation is running:

* Press `F12` to toggle full screen.
* Press `Space`, `Page Down`, or left-click to advance the resolution process.

To show a collective award page after each medal batch, set `batch_award_enabled = true` in the CDP's `config.toml`. This feature is disabled by default and is independent of automatic resolution:

```toml
[presentation]
skip_no_medal = true
intro_scroll_seconds = 8.0
batch_award_enabled = true
auto_advance_enabled = false
auto_advance_interval_seconds = 1.0
auto_advance_award_interval_seconds = 5.0
batch_name_scroll_cycle_seconds = 30.0
batch_name_width_ratio = 0.5
```

The batch size is set per award, not globally: set a positive batch size on the gold, silver, and bronze medals in the "Set Medal" stage (or in `awards.json` as each award's `batch_size` field). An award with no batch size (or `0`) is shown one recipient at a time instead of being grouped.

After startup, with `auto_advance_enabled = false`, use a left-click, `Space`, or `Page Down` to advance each resolution step and individual award page; the collective award page appears after the last individual award page of each batch is dismissed. With both `batch_award_enabled = true` and `auto_advance_enabled = true`, the presentation waits for manual input before each batch, then resolves submissions, updates rankings, and shows each team's award page automatically. Only teams receiving a bronze, silver, or gold medal whose award has a positive batch size count toward the batch size; extra awards for the same team do not add to the count. Both the GUI's IDs (`medal-bronze`, `medal-silver`, `medal-gold`) and DOMjudge's IDs (`bronze-medal`, `silver-medal`, `gold-medal`) are recognized. GUI medal assignments take precedence for each tier when both forms exist. Teams without pending submissions still receive their awards.

The top three finishers are the champion, runner-up, and third place. They are gold medalists (the gold count includes them), but they are shown separately — one recipient at a time, outside of the gold batches. Each page lists their finishing title (`Champion` / `Runner-up` / `Third place`), `Gold Medal`, and any other non-medal awards they won. Each of these award pages advances manually: a click is required to leave one award page and move to the next.

With `skip_no_medal = true`, the opening browse is followed by visible automatic resolution of the rows below the last medal rank, even when `auto_advance_enabled = false`. The cutoff is calculated once from the lowest gold, silver, or bronze recipient in the final standings, filtered to teams included in the presentation. This preserves the original skip boundary: for example, if the last medal rank is 80, rows 200 through 81 are resolved before pausing at row 80. The cutoff is a row index, not a team identity; an eventual medalist still below the cutoff must have its pending submissions revealed and move upward as necessary. Custom awards and the implicit top-three finishing awards do not define the cutoff. During this automatic lead-in, all award pages and batch collection are suppressed, including custom awards belonging to teams below the cutoff.

The lead-in uses the normal reveal, resort, and movement animations and `auto_advance_interval_seconds`; manual advancement is ignored. Any pending resort completes before checking the cutoff. When focus reaches the cutoff row, the presentation pauses before processing that row. A left-click, `Space`, or `Page Down` releases this pause and performs the next normal step; subsequent advancement follows the configured automatic/manual and batch settings. If there are no standard medal recipients in the final standings, the opening browse still runs but no automatic lead-in is started and focus stays unset, matching the original no-medal behavior.

With `skip_no_medal = false`, the same frozen-scoreboard opening browse still runs, then normal resolution starts at the lowest row with pending submissions or an award. Automatic mode follows its existing kickoff and award-page pause rules; manual mode waits for input. The opening browse never silently pre-resolves or removes teams.

When automatic resolution is enabled, `auto_advance_interval_seconds` controls ordinary resolution steps (default: 1 second per logical step, not per team). Revealing a pending problem, applying a resort, and advancing to the next team can each take a separate step. Ordinary automatic steps follow this interval without waiting for scrolling or row-movement animations to finish, like repeated manual advancement; the animations keep their independently configured durations. Award-page timing and manual pauses at batch boundaries and the top three are unchanged. `auto_advance_award_interval_seconds` controls how long each team's photo and award page stays fully visible after the fade-in (default: 5 seconds). With collective award pages enabled, the last award page of a batch also uses this interval, then closes automatically so the presentation pauses on the collective award page. Outside the automatic below-medal-boundary lead-in described above, neither interval advances pages in manual mode.

Batch endpoints are calculated when the presentation launches, using the current medal assignments (including a reloaded `awards.json`) and final standings, from the lowest rank upward within each tier. Each batch pauses when the requested number of medals has been shown **or that medal tier is finished**, whichever comes first. For example, 23 bronze recipients with a batch size of 12 produce batches of 12 and 11. Each batch pauses on the collective award page. The next manual input dismisses it and resumes manual stepping, or starts the next automatic batch if automatic resolution is enabled. Silver starts with a fresh count.

The collective award page shows the actual recipients of that batch in reveal order, with circular school logos and team photos on either side of a wide name list. Names use large text and natural line heights without shrinking to fit the column, with a thin divider after each complete name. When the list fits, it stays centered and still; when it overflows, it pauses for two seconds before scrolling upward in a continuous loop. Each new batch starts from the first name. The side columns contain no text, and the middle has no medal or team-count heading. Photos have no background fill. Missing or unreadable team photos are hidden, including their frames; this page does not substitute fallback images or school logos for photos. The page fades in and out over 350 ms. The next left-click, `Space`, or `Page Down` fades it out, then returns to the scoreboard and resumes stepping. After the final batch, manual input fades back to the completed scoreboard. Additional input during a transition or a running automatic batch is ignored.

Setting `batch_award_enabled = false` disables collective award pages and batch-based automatic execution, even if medals have positive batch sizes. With this setting disabled, or if no eligible medal recipient has a positive batch size, automatic resolution pauses at every individual award page as before. Setting `auto_advance_enabled = false` keeps manual stepping without disabling collective award pages. Existing configurations that used automatic batches must explicitly add `batch_award_enabled = true` to retain that behavior.

`batch_name_scroll_cycle_seconds` sets the time for one complete loop of all names on the collective award page (default: 30 seconds, range: 0.1–3600 seconds). The scroll speed is the measured height of one complete name list divided by this duration, including wrapped lines, dividers and spacing. The initial two-second pause is separate. A larger value scrolls more slowly; lists that fit stay still.

`batch_name_width_ratio` controls the collective award page's middle name column (default: `0.5`, range: `0.1`–`0.9`). After subtracting the outer margins and the two column gaps, the middle column receives this fraction of the available width; the left and right image columns each receive half of the remainder. For example, `0.6` gives 60% to the middle and 20% to each side. The middle column's padding and borders are included in that fraction, so the text itself has slightly less space. Side cards use the full allocated column width and scale down to fit narrow columns without spilling into the names. Names still wrap at the available text width. The canvas width follows the viewport's aspect ratio so taller batches do not leave large empty bands on either side; images and text retain uniform scaling. Nonfinite or nonnumeric values use the default; numeric values outside the range are clamped.

Animation speed is configured in the `config.toml` file and cannot be changed during the presentation. Be sure to test everything beforehand.

Main logic is shown as following state machine.

![presentation flow](Docs/presentation_workflow.svg)

## Build

### Windows Native AOT static linking

For a fully static Windows AOT build, first place the required static libraries inside the `Native` directory:

* [https://github.com/2ndlab/ANGLE.Static](https://github.com/2ndlab/ANGLE.Static)
* [https://github.com/2ndlab/SkiaSharp.Static](https://github.com/2ndlab/SkiaSharp.Static)

The expected files are:

```text
Native\libHarfBuzzSharp.lib
Native\skia.lib
Native\SkiaSharp.lib
Native\av_libglesv2.lib
```

Then uncomment the `ItemGroup` labeled `ImportLib` in `Pyrite.csproj` so the AOT publish can link these libraries through `DirectPInvoke` / `NativeLibrary`.

From a Windows PowerShell terminal, run:

```powershell
dotnet publish .\Pyrite.csproj -c Release -r win-x64 -p:PublishAot=true --self-contained true
```

To publish into a fixed output directory, run:

```powershell
dotnet publish .\Pyrite.csproj -c Release -r win-x64 -p:PublishAot=true --self-contained true -o .\out\win-x64-aot
```

`Pyrite.csproj` already sets `PublishAot` to `true`, but keeping `-p:PublishAot=true` in the command makes the release intent explicit. The published executable will be under the selected output directory, or under `bin\Release\net10.0\win-x64\publish` when `-o` is not specified.
