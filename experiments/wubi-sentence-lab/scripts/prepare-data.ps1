param(
    [string]$QingjianDirectory = (Join-Path $env:ProgramFiles 'Qingjian'),
    [string]$LanguageModel
)
$ErrorActionPreference = 'Stop'
$labRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = (Resolve-Path (Join-Path $labRoot '../..')).Path
$table = Join-Path $repoRoot 'assets/schemes/wubi86-sentence/table.tsv'
if (-not $LanguageModel) { $LanguageModel = Join-Path $QingjianDirectory 'data/generated/lm.qj' }
if (-not (Test-Path -LiteralPath $LanguageModel -PathType Leaf)) {
    throw 'lm.qj not found. Pass -LanguageModel with a model prepared from Qingjian data, or -QingjianDirectory with an existing installation.'
}
$data = Join-Path $labRoot 'data'
New-Item -ItemType Directory -Path $data -Force | Out-Null
Copy-Item -LiteralPath $table -Destination (Join-Path $data 'wubi.tsv') -Force
Copy-Item -LiteralPath $LanguageModel -Destination (Join-Path $data 'lm.qj') -Force
'Prepared experiment data; existing Qingjian configuration and installation were not modified.'
