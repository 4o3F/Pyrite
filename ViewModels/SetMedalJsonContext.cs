using Pyrite.Models;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Pyrite.ViewModels;

[JsonSourceGenerationOptions(PropertyNameCaseInsensitive = true, WriteIndented = true)]
[JsonSerializable(typeof(Dictionary<string, Award>))]
[JsonSerializable(typeof(ContestState))]
internal sealed partial class SetMedalJsonContext : JsonSerializerContext
{
}
