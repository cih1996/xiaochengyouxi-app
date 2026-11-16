param(
    [string]$BaseUrl='http://127.0.0.1:38180',
    [string]$AdminPassword=$env:ORANGE_TEST_ADMIN_PASSWORD,
    [string]$AdminUsername=$env:ORANGE_TEST_ADMIN_USERNAME,
    [string]$AdminToken=$env:ORANGE_TEST_ADMIN_TOKEN,
    [string]$CsjSecret=$env:ORANGE_TEST_CSJ_SECRET
)
$ErrorActionPreference='Stop'
$BaseUrl=$BaseUrl.TrimEnd('/')
if (-not $AdminUsername) {$AdminUsername='admin'}
$checks=0
function Assert($Condition,[string]$Message) {
    if (-not $Condition) { throw "FAILED: $Message" }
    $script:checks++
    Write-Host "PASS $Message"
}
function Call([string]$Method,[string]$Path,$Body=$null,[string]$Token='') {
    $params=@{Method=$Method;Uri="$BaseUrl$Path";ContentType='application/json'}
    $params.Headers=@{}
    if ($Token) {$params.Headers.Authorization="Bearer $Token"}
    if ($script:AdminToken -and $Path.StartsWith('/api/admin/') -and $Path -ne '/api/admin/login') {$params.Headers['x-admin-token']=$script:AdminToken}
    if ($null -ne $Body) {$params.Body=ConvertTo-Json $Body -Depth 8 -Compress}
    Invoke-RestMethod @params
}
function ExpectStatus([int]$Status,[scriptblock]$Action,[string]$Message) {
    try { & $Action | Out-Null; throw "Expected HTTP $Status" }
    catch {
        if ($null -eq $_.Exception.Response) {throw}
        Assert ([int]$_.Exception.Response.StatusCode -eq $Status) $Message
    }
}
function BindInvitation($Account) {
    if ($Account.user.invitation_required) {
        $platformCode=(Call GET '/api/admin/invitations').platform_code
        $Account.user=(Call POST '/api/invite/bind' @{invite_code=$platformCode} $Account.token).user
    }
    Assert (-not $Account.user.invitation_required) 'test account invitation bound'
}
function StartReward([string]$Task,[string]$Token) {
    $quote=Call POST '/api/rewards/quote' @{task_id=$Task} $Token
    Call POST '/api/rewards/start' @{task_id=$Task;quote_id=$quote.quote_id} $Token
}
function ReadHall([string]$Token,[int]$Seconds,[scriptblock]$OnItem=$null) {
    $client=[System.Net.Http.HttpClient]::new()
    $client.DefaultRequestHeaders.Authorization=[System.Net.Http.Headers.AuthenticationHeaderValue]::new('Bearer',$Token)
    $response=$client.GetAsync("$BaseUrl/api/hall/stream",[System.Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult()
    $reader=$null
    try {
        Assert ([int]$response.StatusCode -eq 200) 'SSE connected'
        Assert ($response.Content.Headers.ContentType.MediaType -eq 'text/event-stream') 'SSE content type'
        $reader=[System.IO.StreamReader]::new($response.Content.ReadAsStreamAsync().GetAwaiter().GetResult())
        $clock=[System.Diagnostics.Stopwatch]::StartNew()
        while ($clock.Elapsed.TotalSeconds -lt $Seconds) {
            $lineTask=$reader.ReadLineAsync()
            $remaining=[Math]::Max(1,[int](($Seconds-$clock.Elapsed.TotalSeconds)*1000))
            if (-not $lineTask.Wait($remaining)) {break}
            $line=$lineTask.Result
            if ($null -eq $line) {break}
            if ($line.StartsWith('data:')) {
                $item=ConvertFrom-Json $line.Substring(5).Trim()
                if ($OnItem) {& $OnItem $item | Out-Null}
                $item
            }
        }
    } finally {
        if ($reader) {$reader.Dispose()}
        $response.Dispose()
        $client.Dispose()
    }
}

Assert ((Call GET '/api/health').ok) 'server health'
$config=Call GET '/api/config'
Assert ($config.sms_mode -eq 'mock' -and $config.payment_mode -eq 'mock') 'public runtime modes'
if ($config.device_trust_enabled) {throw 'This isolated smoke fixture requires DeviceTrust disabled; use tools/verify-sdk05-rewards.py for signed SDK integration.'}
if ($config.admin_auth_required) {
    ExpectStatus 401 {Invoke-RestMethod "$BaseUrl/api/admin/overview"} 'admin GET requires token'
    ExpectStatus 401 {Invoke-RestMethod "$BaseUrl/api/admin/settings" -Method PUT -ContentType 'application/json' -Body '{"min_withdraw_yuan":1,"daily_reward_limit":20,"reward_amount":100}'} 'admin PUT requires token'
    ExpectStatus 401 {Call POST '/api/admin/login' @{username=$AdminUsername;password='definitely-wrong-admin-password'}} 'wrong admin password rejected'
    if (-not $AdminPassword) {throw 'Set ORANGE_TEST_ADMIN_PASSWORD for authenticated smoke checks'}
    $adminLogin=Call POST '/api/admin/login' @{username=$AdminUsername;password=$AdminPassword}
    Assert ([bool]$adminLogin.token) 'admin password login'
    $script:AdminToken=$adminLogin.token
    Assert ([bool](Call GET '/api/admin/settings')) 'admin token authorizes request'
}
if ($config.ad_mode -eq 'csj') {
    if (-not $CsjSecret) {throw 'Set ORANGE_TEST_CSJ_SECRET for signed callback checks'}
    if ($config.environment -eq 'production') {
        ExpectStatus 401 {Call POST '/api/auth/login' @{phone='13800138000';password='Orange123!'}} 'production does not seed demo credentials'
    }
    $phone='139'+(Get-Random -Minimum 10000000 -Maximum 99999999)
    Call POST '/api/auth/sms' @{phone=$phone;purpose='register'} | Out-Null
    $account=Call POST '/api/auth/register' @{phone=$phone;password='TestPass123!';code='123456'}
    BindInvitation $account
    $token=$account.token
    Assert ($account.user.coins -eq 0) 'CSJ test account starts empty'
    $otherPhone='137'+(Get-Random -Minimum 10000000 -Maximum 99999999)
    Call POST '/api/auth/sms' @{phone=$otherPhone;purpose='register'} | Out-Null
    $other=Call POST '/api/auth/register' @{phone=$otherPhone;password='TestPass123!';code='123456'}
    BindInvitation $other
    $session=StartReward 'welcome' $token
    Assert ($session.mode -eq 'csj' -and $session.ad_app_id -eq '5931950' -and $session.ad_code_id -eq '993606639' -and $null -eq $session.duration_seconds) 'CSJ start uses real provider without mock timer'
    Assert ((ConvertFrom-Json $session.media_extra).reward_session_id -eq $session.id -and $session.user_id -eq [string]$account.user.id) 'CSJ owner and canonical media extra'
    ExpectStatus 409 {Call POST "/api/rewards/$($session.id)/complete" @{} $token} 'unverified CSJ completion rejected'
    ExpectStatus 404 {Call POST "/api/rewards/$($session.id)/complete" @{} $other.token} 'reward completion ownership enforced'
    $providerSettings=Call GET '/api/admin/settings'
    $providerName=$providerSettings.csj_expected_reward_name
    $providerAmount=[int]$providerSettings.csj_expected_reward_amount
    function Callback([string]$TransId,[string]$SessionExtra,[string]$UserId,[string]$SignOverride='', [int]$Amount=$providerAmount) {
        $signature=[Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData([System.Text.Encoding]::UTF8.GetBytes("${CsjSecret}:$TransId"))).ToLowerInvariant()
        if ($SignOverride) {$signature=$SignOverride}
        $values=@{user_id=$UserId;trans_id=$TransId;reward_name=$providerName;reward_amount=[string]$Amount;extra=$SessionExtra;sign=$signature}
        $query=($values.GetEnumerator() | ForEach-Object {"$($_.Key)=$([Uri]::EscapeDataString($_.Value))"}) -join '&'
        Call GET "/csj/callback?$query"
    }
    $transId=[guid]::NewGuid().ToString()
    Assert (-not (Callback $transId $session.media_extra $session.user_id ('0'*64)).isValid) 'forged callback signature rejected'
    Assert (-not (Callback $transId '{"reward_session_id":"missing-session"}' $session.user_id).isValid) 'unknown callback session rejected'
    Assert (-not (Callback $transId $session.media_extra ([string]$other.user.id)).isValid) 'callback owner mismatch rejected'
    Assert (-not (Callback $transId $session.media_extra $session.user_id '' 0).isValid) 'non-positive platform reward rejected'
    Assert (-not (Callback $transId $session.media_extra $session.user_id '' 999999).isValid) 'unexpected provider reward rejected'
    Assert ((Callback $transId $session.media_extra $session.user_id).isValid) 'signed CSJ callback verified with provider metadata'
    Assert ((Call GET '/api/me' $null $token).coins -eq 0) 'callback verification does not credit account'
    Assert ((Callback $transId $session.media_extra $session.user_id).isValid) 'duplicate correct callback idempotent'
    $second=StartReward 'lucky' $token
    Assert (-not (Callback $transId $second.media_extra $second.user_id).isValid) 'platform transaction cannot verify another session'
    $reward=Call POST "/api/rewards/$($session.id)/complete" @{} $token
    Assert ($reward.user.coins -eq $session.reward -and $reward.reward -eq $session.reward) 'verified completion uses configured snapshot reward'
    $repeat=Call POST "/api/rewards/$($session.id)/complete" @{} $token
    Assert ($repeat.already_completed -and $repeat.user.coins -eq $reward.user.coins) 'CSJ completion credits exactly once'
    $ledger=Call GET '/api/ledger' $null $token
    Assert ($ledger.Count -eq 1) 'one CSJ credit ledger entry'
    $events=@(ReadHall $token 2)
    Assert ($events.Count -ge 2 -and $events[1].kind -eq 'red_packet') 'production authenticated hall stream'
    $public=Call GET '/api/config' | ConvertTo-Json -Compress
    Assert (-not $public.Contains($CsjSecret) -and -not $public.Contains($script:AdminToken)) 'public configuration excludes secrets'
    Write-Host "Completed $checks CSJ/API checks. Test account: $phone"
    return
}
ExpectStatus 401 {Call GET '/api/me'} 'profile requires session'
ExpectStatus 401 {Call POST '/api/auth/login' @{phone='13800138000';password='wrong-password'}} 'wrong password rejected'
$guestTasks=Call GET '/api/tasks'
Assert ($guestTasks.Count -eq 3) 'guest task preview'
$phone='139'+(Get-Random -Minimum 10000000 -Maximum 99999999)
$sms=Call POST '/api/auth/sms' @{phone=$phone;purpose='register'}
Assert ($sms.code -eq '123456') 'mock SMS code'
ExpectStatus 400 {Call POST '/api/auth/register' @{phone=$phone;password='TestPass123!';code='000000'}} 'invalid code rejected'
$account=Call POST '/api/auth/register' @{phone=$phone;password='TestPass123!';code='123456';nickname='联调用户'}
BindInvitation $account
$token=$account.token
Assert ($account.user.coins -eq 0 -and $account.user.phone -eq $phone) 'registered zero-balance account'
$login=Call POST '/api/auth/login' @{phone=$phone;password='TestPass123!'}
Assert ([bool]$login.token) 'password login'
Call POST '/api/auth/sms' @{phone=$phone;purpose='register'} | Out-Null
ExpectStatus 409 {Call POST '/api/auth/register' @{phone=$phone;password='TestPass123!';code='123456'}} 'duplicate account rejected'
$signin=Call POST '/api/check-in' @{} $token
Assert ($signin.reward -eq 50 -and $signin.user.coins -eq 50) 'check-in credits ledger'
ExpectStatus 409 {Call POST '/api/check-in' @{} $token} 'daily check-in cannot duplicate'
$session=StartReward 'welcome' $token
ExpectStatus 400 {Call POST "/api/rewards/$($session.id)/complete" @{} $token} 'reward requires elapsed playback'
$demo=Call POST '/api/auth/login' @{phone='13800138000';password='Orange123!'}
BindInvitation $demo
ExpectStatus 404 {Call POST "/api/rewards/$($session.id)/complete" @{} $demo.token} 'reward ownership checked'
Start-Sleep -Seconds 6
$reward=Call POST "/api/rewards/$($session.id)/complete" @{} $token
Assert ($reward.user.coins -eq (50+$session.reward)) 'reward credits account'
$repeat=Call POST "/api/rewards/$($session.id)/complete" @{} $token
Assert ($repeat.already_completed -and $repeat.user.coins -eq $reward.user.coins) 'reward completion idempotent'
$ledger=Call GET '/api/ledger' $null $token
Assert ($ledger.Count -eq 2) 'one entry per credit'
ExpectStatus 400 {Call POST '/api/withdrawals' @{amount_yuan=0.1;channel='alipay';account='test@example.com'} $token} 'withdrawal threshold enforced'
$before=(Call GET '/api/me' $null $demo.token).coins
$withdrawal=Call POST '/api/withdrawals' @{amount_yuan=1;channel='alipay';account='local-demo'} $demo.token
Assert ($withdrawal.user.coins -eq ($before-1000) -and $withdrawal.status -eq '模拟申请') 'mock withdrawal debits once'
$refunded=Call PUT "/api/admin/withdrawals/$($withdrawal.id)" @{status='rejected'}
Assert ($refunded.user.coins -eq $before) 'mock rejection refunds once'
ExpectStatus 409 {Call PUT "/api/admin/withdrawals/$($withdrawal.id)" @{status='rejected'}} 'withdrawal cannot refund twice'
$products=Call GET '/api/products'
Assert ($products.Count -eq 4) 'catalog demo products'
$settings=Call GET '/api/admin/settings'
Assert ($settings.coins_per_yuan -eq 1000 -and $settings.sms_mode -eq 'mock') 'local settings'
ExpectStatus 401 {Call GET '/api/hall/stream'} 'hall stream requires session'
ExpectStatus 404 {Call GET '/api/messages'} 'old history route absent'
$strategy=Call GET '/api/admin/hall-strategy'
Assert ($strategy.enabled -and $strategy.interval_seconds -eq 20 -and $strategy.video_weight -eq 70 -and $strategy.product_weight -eq 30) 'hall strategy defaults'
$events=@(ReadHall $token 8)
Assert ($events.Count -eq 3 -and $events[0].kind -eq 'text' -and $events[1].kind -eq 'red_packet' -and $events[2].kind -eq 'product') 'welcome then server-pushed packet and product'
Assert ([bool]$events[1].task.id -and [bool]$events[2].product.id -and $events[0].id -ne $events[1].id) 'SSE card payloads and unique event IDs'
$strategy.enabled=$false
$saved=Call PUT '/api/admin/hall-strategy' $strategy
Assert (-not $saved.enabled) 'strategy saved'
$silent=@(ReadHall $token 2)
Assert ($silent.Count -eq 0) 'disabled hall emits no items'
$strategy.enabled=$true
$strategy.video_weight=0
$strategy.product_weight=100
$strategy.interval_seconds=1
Call PUT '/api/admin/hall-strategy' $strategy | Out-Null
$productEvents=@(ReadHall $token 8)
Assert (($productEvents | Where-Object kind -eq 'red_packet').Count -eq 0 -and ($productEvents | Where-Object kind -eq 'product').Count -ge 1) 'zero video weight only recommends products'
$strategy.video_weight=70
$strategy.product_weight=30
$strategy.interval_seconds=20
Call PUT '/api/admin/hall-strategy' $strategy | Out-Null
$liveEvents=@(ReadHall $token 8 {
    param($item)
    if ($item.kind -eq 'red_packet') {
        $strategy.enabled=$false
        Call PUT '/api/admin/hall-strategy' $strategy | Out-Null
    }
})
Assert ($liveEvents.Count -eq 2 -and ($liveEvents | Where-Object kind -eq 'product').Count -eq 0) 'strategy edits affect existing connection'
$strategy.enabled=$true
Call PUT '/api/admin/hall-strategy' $strategy | Out-Null
$overview=Call GET '/api/admin/overview'
Assert ($overview.total_users -ge 2 -and $overview.rewards -ge 1) 'admin statistics reflect activity'
Call POST '/api/auth/sms' @{phone=$phone;purpose='reset'} | Out-Null
Call POST '/api/auth/reset-password' @{phone=$phone;code='123456';password='UpdatedPass123!'} | Out-Null
ExpectStatus 401 {Call GET '/api/me' $null $token} 'password reset invalidates old sessions'
ExpectStatus 401 {Call POST '/api/auth/login' @{phone=$phone;password='TestPass123!'}} 'old password rejected'
$updated=Call POST '/api/auth/login' @{phone=$phone;password='UpdatedPass123!'}
Assert ([bool]$updated.token) 'new password works'
$logoutEvents=@(ReadHall $updated.token 4 {
    param($item)
    if ($item.kind -eq 'text') {Call POST '/api/auth/logout' @{} $updated.token | Out-Null}
})
Assert ($logoutEvents.Count -eq 2 -and $logoutEvents[1].status -eq 401 -and [bool]$logoutEvents[1].error) 'logout sends actionable SSE error and closes connection'
ExpectStatus 401 {Call GET '/api/me' $null $updated.token} 'logout invalidates token'
ExpectStatus 404 {Call GET '/api/not-an-endpoint'} 'unknown API is JSON 404'
Write-Host "Completed $checks API checks. Test account: $phone"
