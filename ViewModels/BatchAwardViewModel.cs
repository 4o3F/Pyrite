using Avalonia.Media.Imaging;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Pyrite.ViewModels;

public sealed class BatchAwardViewModel : IDisposable
{
    public BatchAwardViewModel(IReadOnlyList<BatchAwardTeamViewModel> teams, double nameScrollCycleSeconds,
        double nameWidthRatio = 0.5)
    {
        Teams = teams;
        NameScrollCycleSeconds = double.IsFinite(nameScrollCycleSeconds)
            ? Math.Clamp(nameScrollCycleSeconds, 0.1, 3600)
            : 30;
        NameWidthRatio = double.IsFinite(nameWidthRatio) ? Math.Clamp(nameWidthRatio, 0.1, 0.9) : 0.5;
        var split = (teams.Count + 1) / 2;
        LeftTeams = teams.Take(split).ToList();
        RightTeams = teams.Skip(split).ToList();
    }

    public IReadOnlyList<BatchAwardTeamViewModel> Teams { get; }
    public IReadOnlyList<BatchAwardTeamViewModel> LeftTeams { get; }
    public IReadOnlyList<BatchAwardTeamViewModel> RightTeams { get; }
    public double NameScrollCycleSeconds { get; }
    public double NameWidthRatio { get; }
    // Keep every recipient visible even when the configured batch is larger than twelve.
    public double CanvasHeight => Math.Max(1080, ((Teams.Count + 1) / 2) * 160 + 96);

    public void Dispose()
    {
        foreach (var team in Teams)
            team.Dispose();
    }
}

public sealed class BatchAwardTeamViewModel : IDisposable
{
    public BatchAwardTeamViewModel(string teamName, Bitmap? affiliationLogo, Bitmap? photo)
    {
        TeamName = teamName;
        AffiliationLogoImage = affiliationLogo;
        TeamPhotoImage = photo;
    }

    public string TeamName { get; }
    public Bitmap? AffiliationLogoImage { get; }
    public Bitmap? TeamPhotoImage { get; }
    public bool HasTeamPhoto => TeamPhotoImage is not null;

    public void Dispose()
    {
        // Logos belong to the presentation's cache; only thumbnails are owned by this page.
        TeamPhotoImage?.Dispose();
    }
}
