using Pyrite.Models;
using System.Text.Json.Serialization;

namespace Pyrite.Services;

[JsonSourceGenerationOptions(PropertyNameCaseInsensitive = true)]
[JsonSerializable(typeof(Event))]
[JsonSerializable(typeof(Contest))]
[JsonSerializable(typeof(JudgementType))]
[JsonSerializable(typeof(Group))]
[JsonSerializable(typeof(Organization))]
[JsonSerializable(typeof(Team))]
[JsonSerializable(typeof(Account))]
[JsonSerializable(typeof(Problem))]
[JsonSerializable(typeof(Submission))]
[JsonSerializable(typeof(Judgement))]
[JsonSerializable(typeof(Award))]
internal sealed partial class EventFeedJsonContext : JsonSerializerContext
{
}
