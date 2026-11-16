param([string]$BaseUrl='http://127.0.0.1:39185',[string]$AdminPassword=$env:ORANGE_TEST_ADMIN_PASSWORD)
$ErrorActionPreference='Stop'
$BaseUrl=$BaseUrl.TrimEnd('/')
$checks=0
$adminToken=''
function Assert($Condition,[string]$Message) {
    if (-not $Condition) {throw "FAILED: $Message"}
    $script:checks++
    Write-Host "PASS $Message"
}
function Call([string]$Method,[string]$Path,$Body=$null,[string]$Token='') {
    $p=@{Method=$Method;Uri="$BaseUrl$Path";ContentType='application/json';Headers=@{}}
    if ($Token) {$p.Headers.Authorization="Bearer $Token"}
    if ($script:adminToken -and $Path.StartsWith('/api/admin/') -and $Path -ne '/api/admin/login') {$p.Headers['x-admin-token']=$script:adminToken}
    if ($null -ne $Body) {$p.Body=ConvertTo-Json $Body -Depth 8 -Compress}
    Invoke-RestMethod @p
}
function Expect([int]$Status,[scriptblock]$Action,[string]$Message,[string]$MachineCode='') {
    try {& $Action | Out-Null;throw "Expected HTTP $Status"}
    catch {
        if ($null -eq $_.Exception.Response) {throw}
        Assert ([int]$_.Exception.Response.StatusCode -eq $Status) $Message
        if ($MachineCode) {Assert ((ConvertFrom-Json $_.ErrorDetails.Message).code -eq $MachineCode) "$Message machine code"}
    }
}
function Phone { '139'+(Get-Random -Minimum 10000000 -Maximum 99999999) }
function Register([string]$PhoneNumber) {
    Call POST '/api/auth/sms' @{phone=$PhoneNumber;purpose='register'} | Out-Null
    Call POST '/api/auth/register' @{phone=$PhoneNumber;password='InviteTest123!';code='123456'}
}
function ValidCode([string]$Code) {
    $Code -cmatch '^[A-HJ-NP-Z2-9]{6}$' -and $Code -cmatch '[A-Z]' -and $Code -match '[2-9]'
}
function ReadHall([string]$Token,[int]$Seconds,[string]$Suffix='') {
    $client=[System.Net.Http.HttpClient]::new()
    $client.DefaultRequestHeaders.Authorization=[System.Net.Http.Headers.AuthenticationHeaderValue]::new('Bearer',$Token)
    $response=$client.GetAsync("$BaseUrl/api/hall/stream$Suffix",[System.Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult()
    $reader=$null
    try {
        Assert ([int]$response.StatusCode -eq 200) 'bound SSE connection'
        $reader=[System.IO.StreamReader]::new($response.Content.ReadAsStreamAsync().GetAwaiter().GetResult())
        $clock=[System.Diagnostics.Stopwatch]::StartNew()
        while ($clock.Elapsed.TotalSeconds -lt $Seconds) {
            $lineTask=$reader.ReadLineAsync()
            if (-not $lineTask.Wait([Math]::Max(1,[int](($Seconds-$clock.Elapsed.TotalSeconds)*1000)))) {break}
            $line=$lineTask.Result
            if ($null -eq $line) {break}
            if ($line.StartsWith('data:')) {ConvertFrom-Json $line.Substring(5).Trim()}
        }
    } finally {
        if ($reader) {$reader.Dispose()}
        $response.Dispose()
        $client.Dispose()
    }
}

$config=Call GET '/api/config'
Assert ($config.invite_required -and -not $config.invite_sms_required) 'invitation gate and phone-only landing configuration'
Assert ($config.app_name -eq '小橙有喜' -and $config.android_package -eq 'com.lingzidev.xiaochengyouxi') 'formal app identity'
if ($config.admin_auth_required) {
    Expect 401 {Call GET '/api/admin/invitations'} 'invitation admin protected'
    $script:adminToken=(Call POST '/api/admin/login' @{password=$AdminPassword}).token
}
$admin=Call GET '/api/admin/invitations'
$platform=$admin.platform_code
Assert (ValidCode $platform) 'platform invitation code format'
Assert (-not ((ConvertTo-Json $config -Compress).Contains($platform))) 'platform code only disclosed in management'

$a=Register (Phone)
$b=Register (Phone)
Assert ($a.user.invitation_required -and (ValidCode $a.user.invite_code) -and (ValidCode $b.user.invite_code) -and $a.user.invite_code -ne $b.user.invite_code) 'direct registration receives unique mixed invitation code'
Assert ((Call GET '/api/me' $null $a.token).invitation_required) 'partial session may inspect profile'
foreach ($path in @('/api/tasks','/api/products','/api/ledger','/api/withdrawals','/api/invite/summary','/api/hall/stream')) {
    Expect 403 {Call GET $path $null $a.token} "unbound denied $path" 'INVITE_REQUIRED'
}
foreach ($item in @(
    @{path='/api/rewards/start';body=@{task_id='welcome'}},
    @{path='/api/rewards/nonexistent/complete';body=@{}},
    @{path='/api/check-in';body=@{}},
    @{path='/api/withdrawals';body=@{amount_yuan=1;channel='alipay';account='test'}}
)) {Expect 403 {Call POST $item.path $item.body $a.token} "unbound denied $($item.path)" 'INVITE_REQUIRED'}
Assert ((Call GET '/api/tasks').Count -eq 3 -and (Call GET '/api/products').Count -eq 4) 'anonymous catalog remains public'
Call POST '/api/auth/logout' @{} $a.token | Out-Null
Expect 401 {Call GET '/api/me' $null $a.token} 'partial session can log out'
$a=Call POST '/api/auth/login' @{phone=$a.user.phone;password='InviteTest123!'}
Assert ($a.user.invitation_required) 'direct password login stays gated'
Expect 400 {Call POST '/api/invite/bind' @{invite_code=$a.user.invite_code} $a.token} 'self invitation rejected'
Expect 400 {Call POST '/api/invite/bind' @{invite_code=$b.user.invite_code} $a.token} 'unbound inviter cannot recruit'
Expect 400 {Call POST '/api/invite/bind' @{invite_code='O0I1AA'} $a.token} 'ambiguous invitation characters rejected'
$a.user=(Call POST '/api/invite/bind' @{invite_code=$platform} $a.token).user
Assert (-not $a.user.invitation_required -and $a.user.inviter_code -eq $platform -and $a.user.coins -eq 0) 'platform binding unlocks without fake bonus'
Assert (-not (Call POST '/api/invite/bind' @{invite_code=$platform} $a.token).user.invitation_required) 'same binding retry is idempotent'
$b.user=(Call POST '/api/invite/bind' @{invite_code=$a.user.invite_code} $b.token).user
Expect 409 {Call POST '/api/invite/bind' @{invite_code=$b.user.invite_code} $a.token} 'bound relation cannot change or form a cycle'
$summary=Call GET '/api/invite/summary' $null $a.token
Assert ($summary.registered_count -eq 1 -and $summary.pending_count -eq 0) 'direct invitation counted after binding'

$cPhone=Phone
$pending=Call POST '/api/invite/prebind' @{phone=$cPhone;invite_code=$a.user.invite_code}
Assert ($pending.status -eq 'pending' -and $pending.invite_code -eq $a.user.invite_code) 'public phone-only prebinding'
Assert ((Call POST '/api/invite/prebind' @{phone=$cPhone;invite_code=$a.user.invite_code}).status -eq 'pending') 'same first-touch prebinding idempotent'
Expect 409 {Call POST '/api/invite/prebind' @{phone=$cPhone;invite_code=$platform}} 'different prebinding source cannot overwrite'
$summary=Call GET '/api/invite/summary' $null $a.token
Assert ($summary.pending_count -eq 1 -and $summary.registered_count -eq 1 -and $summary.invited_count -eq 2) 'pending phone distinct from registered count'
$c=Register $cPhone
Assert (-not $c.user.invitation_required -and $c.user.inviter_code -eq $a.user.invite_code -and $c.user.coins -eq 0) 'registration consumes pending invitation transactionally'
$summary=Call GET '/api/invite/summary' $null $a.token
Assert ($summary.pending_count -eq 0 -and $summary.registered_count -eq 2 -and $summary.invited_count -eq 2) 'consumption converts pending count exactly once'
$d=Register (Phone)
Call POST '/api/invite/prebind' @{phone=$d.user.phone;invite_code=$a.user.invite_code} | Out-Null
Expect 409 {Call POST '/api/invite/bind' @{invite_code=$platform} $d.token} 'manual binding preserves earlier phone attribution'
$d=Call POST '/api/auth/login' @{phone=$d.user.phone;password='InviteTest123!'}
Assert (-not $d.user.invitation_required -and $d.user.inviter_code -eq $a.user.invite_code) 'password login consumes prebinding for existing account'
Expect 409 {Call POST '/api/invite/prebind' @{phone=$c.user.phone;invite_code=$b.user.invite_code}} 'public landing cannot overwrite bound account'
Expect 409 {Call POST '/api/invite/bind' @{invite_code=$b.user.invite_code} $c.token} 'authenticated account cannot rebind'
Assert ((Call GET '/api/me' $null $c.token).inviter_code -eq $a.user.invite_code) 'original inviter remains unchanged'
Assert ((Call GET '/api/ledger' $null $c.token).Count -eq 0) 'invitations create no invented financial entries'

$qr=Call GET "/api/invite/qr/$($a.user.invite_code)"
Assert ($qr.size -ge 21 -and $qr.modules.Count -eq $qr.size -and ($qr.modules | Where-Object {$_.Length -ne $qr.size -or $_ -match '[^01]'}).Count -eq 0) 'real square QR matrix payload'
Assert ($qr.url -eq $summary.invite_url -and $qr.code -eq $a.user.invite_code) 'QR embeds personalized invitation URL'
$fixture=Join-Path $PSScriptRoot '..\data\invitation-qr-fixture.json'
ConvertTo-Json $qr -Depth 8 | Set-Content -LiteralPath $fixture -Encoding utf8
Assert ((Invoke-WebRequest "$BaseUrl/i/$($a.user.invite_code)").StatusCode -eq 200 -and (Invoke-WebRequest "$BaseUrl/invite").StatusCode -eq 200) 'local invitation SPA routes'

$strategy=Call GET '/api/admin/hall-strategy'
Assert ($strategy.native_ad_weight -eq 15) 'native feed default survives old strategy JSON'
$webEvents=@(ReadHall $a.token 11)
Assert (($webEvents | Where-Object kind -eq 'native_ad').Count -eq 0 -and $webEvents[1].kind -eq 'red_packet') 'ordinary client receives no native feed'
$nativeEvents=@(ReadHall $a.token 11 '?native_ads=1')
if ($config.ad_mode -eq 'csj') {
    $native=@($nativeEvents | Where-Object kind -eq 'native_ad')
    Assert ($native.Count -eq 1 -and $native[0].ad_code_id -eq '993606636' -and $null -eq $native[0].reward) 'capable CSJ client receives real native placement without reward'
} else {
    Assert (($nativeEvents | Where-Object kind -eq 'native_ad').Count -eq 0) 'mock mode never pushes native feed'
}
$admin=Call GET '/api/admin/invitations'
Assert (@($admin.records | Where-Object phone -eq $cPhone)[0].status -eq '已绑定') 'management invitation records reflect binding'
Write-Host "Completed $checks invitation/native checks. QR fixture: $fixture"
