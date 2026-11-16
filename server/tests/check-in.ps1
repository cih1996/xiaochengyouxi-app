param([string]$BaseUrl='http://127.0.0.1:39187',[string]$AdminPassword=$env:ORANGE_TEST_ADMIN_PASSWORD)
$ErrorActionPreference='Stop'
$BaseUrl=$BaseUrl.TrimEnd('/')
$adminToken=''
$checks=0
function Assert($Condition,[string]$Message) {
    if (-not $Condition) {throw "FAILED: $Message"}
    $script:checks++
    Write-Host "PASS $Message"
}
function Call([string]$Method,[string]$Path,$Body=$null,[string]$Token='') {
    $p=@{Method=$Method;Uri="$BaseUrl$Path";ContentType='application/json';Headers=@{}}
    if ($Token) {$p.Headers.Authorization="Bearer $Token"}
    if ($script:adminToken) {$p.Headers['x-admin-token']=$script:adminToken}
    if ($null -ne $Body) {$p.Body=ConvertTo-Json $Body -Compress}
    Invoke-RestMethod @p
}
if ((Call GET '/api/config').admin_auth_required) {$script:adminToken=(Call POST '/api/admin/login' @{password=$AdminPassword}).token}
$platform=(Call GET '/api/admin/invitations').platform_code
$phone='139'+(Get-Random -Minimum 10000000 -Maximum 99999999)
Call POST '/api/auth/sms' @{phone=$phone;purpose='register'} | Out-Null
$account=Call POST '/api/auth/register' @{phone=$phone;password='CheckInTest123!';code='123456'}
Assert ($account.user.checked_in_today -ceq $false) 'registration snapshot starts unchecked'
$bound=Call POST '/api/invite/bind' @{invite_code=$platform} $account.token
Assert ($bound.user.checked_in_today -ceq $false) 'binding snapshot stays unchecked'
Assert ((Call GET '/api/me' $null $account.token).checked_in_today -ceq $false) 'profile before check-in is false'
$result=Call POST '/api/check-in' @{} $account.token
Assert ($result.user.checked_in_today -ceq $true -and $result.user.coins -eq 50) 'check-in snapshot true with one reward'
try {Call POST '/api/check-in' @{} $account.token | Out-Null;throw 'Expected duplicate check-in rejection'}
catch {if ($null -eq $_.Exception.Response) {throw};Assert ([int]$_.Exception.Response.StatusCode -eq 409) 'duplicate check-in remains 409'}
Assert ((Call GET '/api/me' $null $account.token).checked_in_today -ceq $true) 'duplicate attempt preserves checked state'
$login=Call POST '/api/auth/login' @{phone=$phone;password='CheckInTest123!'}
Assert ($login.user.checked_in_today -ceq $true) 'password login restores checked state'
Assert ((Call GET '/api/me' $null $login.token).checked_in_today -ceq $true) 'restored session profile stays checked'
$ledger=Call GET '/api/ledger' $null $login.token
Assert ($ledger.Count -eq 1 -and $ledger[0].coins -eq 50) 'one daily check-in entry and reward'
Write-Host "Completed $checks focused check-in checks."
