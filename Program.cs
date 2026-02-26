using Avalonia;
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Pyrite;

internal sealed class Program
{
    private const uint AttachParentProcess = 0xFFFFFFFF;

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AttachConsole(uint dwProcessId);

    // Initialization code. Don't use any Avalonia, third-party APIs or any
    // SynchronizationContext-reliant code before AppMain is called: things aren't initialized
    // yet and stuff might break.
    [STAThread]
    public static void Main(string[] args)
    {
        // Bind stdout/stderr to `dotnet run` console for runtime logs.
        if (OperatingSystem.IsWindows())
        {
            _ = AttachConsole(AttachParentProcess);
        }

        Trace.Listeners.Add(new ConsoleTraceListener());
        Trace.AutoFlush = true;

        BuildAvaloniaApp()
            .StartWithClassicDesktopLifetime(args);
    }

    // Avalonia configuration, don't remove; also used by visual designer.
    public static AppBuilder BuildAvaloniaApp()
    {
        return AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .WithInterFont()
            .LogToTrace();
    }
}
