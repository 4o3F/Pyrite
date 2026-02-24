using System.Collections.Generic;
using Tomlyn.Model;

namespace Pyrite.Models;

public sealed class PyriteConfig
{
    public List<string> FilterTeamSubmissions { get; set; } = [];
    public Dictionary<string, string> TeamGroupMap { get; set; } = [];
    public PresentationConfig Presentation { get; set; } = new();

    public static PyriteConfig Default()
    {
        return new PyriteConfig();
    }
}

public sealed class PresentationConfig
{
    public int RowsPerPage { get; set; } = 16;
    public float ScrollAnimationSeconds { get; set; } = 0.4f;
    public float RowFlyAnimationSeconds { get; set; } = 0.6f;
    public string LogoExtension { get; set; } = "png";
    public string TeamPhotoExtension { get; set; } = "jpg";
    public string? TeamPhotoFallbackPath { get; set; }

    public static PresentationConfig FromToml(TomlTable table)
    {
        var config = new PresentationConfig();

        if (table.TryGetValue("rows_per_page", out var rowsPerPage) && rowsPerPage is long rows)
            config.RowsPerPage = (int)rows;

        if (table.TryGetValue("scroll_animation_seconds", out var scroll))
            config.ScrollAnimationSeconds = ConvertToFloat(scroll, config.ScrollAnimationSeconds);

        if (table.TryGetValue("row_fly_animation_seconds", out var rowFly))
            config.RowFlyAnimationSeconds = ConvertToFloat(rowFly, config.RowFlyAnimationSeconds);
        else if (table.TryGetValue("row_move_animation_seconds", out var rowMove))
            config.RowFlyAnimationSeconds = ConvertToFloat(rowMove, config.RowFlyAnimationSeconds);

        if (table.TryGetValue("logo_extension", out var logoExtension) && logoExtension is string logo)
            config.LogoExtension = logo;

        if (table.TryGetValue("team_photo_extension", out var teamPhotoExtension) && teamPhotoExtension is string photo)
            config.TeamPhotoExtension = photo;

        if (table.TryGetValue("team_photo_fallback_path", out var fallbackPath) && fallbackPath is string fallback)
            config.TeamPhotoFallbackPath = fallback;

        return config;
    }

    private static float ConvertToFloat(object value, float fallback)
    {
        return value switch
        {
            double d => (float)d,
            long l => l,
            _ => fallback
        };
    }
}