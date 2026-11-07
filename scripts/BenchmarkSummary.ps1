# Summarize complete samples only; never merge versions or memory metrics.
function Get-GPTrayMemorySummary {
    param([object[]]$Rows)
    foreach ($row in $Rows) {
        if (!$row.VersionLabel -or !$row.Scenario) { throw 'VersionLabel and Scenario are required for comparable summaries.' }
    }
    foreach ($group in ($Rows | Group-Object VersionLabel, Scenario)) {
        $valid = @($group.Group | Where-Object { [string]$_.Complete -ieq 'True' })
        $first = $group.Group[0]
        $private = $valid | Measure-Object PrivateWorkingSetMiB -Average -Minimum -Maximum
        $commit = $valid | Measure-Object PrivateCommitMiB -Average
        $working = $valid | Measure-Object WorkingSetMiB -Average
        $count = $valid | Measure-Object ProcessCount -Average -Minimum -Maximum
        [pscustomobject]@{ VersionLabel = $first.VersionLabel; Scenario = $first.Scenario;
            CompleteSamples = $valid.Count; IncompleteSamples = $group.Count - $valid.Count;
            PrivateWorkingSetMeanMiB = $private.Average; PrivateWorkingSetMinMiB = $private.Minimum; PrivateWorkingSetMaxMiB = $private.Maximum;
            PrivateCommitMeanMiB = $commit.Average; WorkingSetMeanMiB = $working.Average;
            ProcessCountMean = $count.Average; ProcessCountMin = $count.Minimum; ProcessCountMax = $count.Maximum }
    }
}
