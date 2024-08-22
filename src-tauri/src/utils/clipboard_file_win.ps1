using namespace 'System.Windows.Forms'
using namespace 'System.Collections.Specialized'; cls
[void][System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms')

$shell = New-Object -ComObject 'WScript.Shell'
$paths = New-Object 'StringCollection'
$arr_err = New-Object 'System.Collections.ArrayList'

$args | foreach { [void]$paths.Add($_) }

$paths = &{
    $paths | Select-Object -Unique | foreach {
        if([System.IO.Directory]::Exists($_)){
            return $_
        }
        if([System.IO.File]::Exists($_)){
            if($_.EndsWith('.lnk')){
                $p = $shell.CreateShortcut($_).TargetPath
                if([System.IO.File]::Exists($p) -or [System.IO.Directory]::Exists($p)){
                    return $p
                } else {
                    [void]$arr_err.Add(("{0} -> {1}" -f $_, $p))
                    return $null
                }
            }
            return $_
        } else {
            [void]$arr_err.Add("error: $_")
            return $null
        }
    } | Select-Object -Unique
}

if($paths.Count -gt 0){
    [Clipboard]::SetFileDropList($paths)
    Write-Host "ok:"
    $paths | ForEach-Object { Write-Host $_ }
}

if($arr_err.Count -gt 0){
    Write-Host
    $arr_err | ForEach-Object { Write-Host $_ }
}
