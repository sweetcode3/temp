# Import required assemblies
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Runtime.WindowsRuntime
Add-Type -Path "$PSScriptRoot\NAudio.dll"

# Set up logging
$logPath = "$PSScriptRoot\logs"
if (-not (Test-Path $logPath)) {
    New-Item -ItemType Directory -Path $logPath
}
$logFile = Join-Path $logPath "bluetooth_monitor.log"

function Write-Log {
    param($Message)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    "$timestamp - $Message" | Add-Content $logFile
    Write-Host "$timestamp - $Message"
}

# Config file path with better error handling
$configPath = "$PSScriptRoot\config.json"

$defaultConfig = @{
    DeviceName = "Your Headphones Name"
    IdleTimeout = 300
    CheckInterval = 30
    AutoReconnect = $true
    LastConfigCheck = Get-Date
    LogRetentionDays = 7
    AudioThreshold = 0.01
    ReconnectAttempts = 3
    ReconnectDelay = 5
}

function Get-Config {
    try {
        if (Test-Path $configPath) {
            $config = Get-Content $configPath -Raw | ConvertFrom-Json
            # Validate config values
            foreach ($key in $defaultConfig.Keys) {
                if ($null -eq $config.$key) {
                    $config.$key = $defaultConfig.$key
                }
            }
            return $config
        }
        $defaultConfig | ConvertTo-Json | Set-Content $configPath
        return $defaultConfig
    }
    catch {
        Write-Log "Error loading config: $_"
        return $defaultConfig
    }
}

function Get-BluetoothDevice {
    param ($deviceName)
    try {
        $bluetoothAPI = [Windows.Devices.Bluetooth.BluetoothAdapter, Windows.System.Runtime, ContentType = WindowsRuntime]
        $bluetooth = $bluetoothAPI::GetDefaultAsync().GetResults()
        if ($null -eq $bluetooth) {
            Write-Log "No Bluetooth adapter found"
            return $null
        }
        $devices = $bluetooth.GetPairedDevicesAsync().GetResults()
        $device = $devices | Where-Object { $_.Name -eq $deviceName }
        if ($null -eq $device) {
            Write-Log "Device '$deviceName' not found"
        }
        return $device
    }
    catch {
        Write-Log "Error getting Bluetooth device: $_"
        return $null
    }
}

function Test-AudioActivity {
    param($threshold)
    $audioMeter = $null
    $waveIn = $null
    try {
        $audioMeter = New-Object NAudio.Wave.MeterStream
        $waveIn = New-Object NAudio.Wave.WaveIn
        $waveIn.StartRecording()
        Start-Sleep -Milliseconds 500
        $peakValue = $audioMeter.PeakValue
        return ($peakValue -gt $threshold)
    }
    catch {
        Write-Log "Error testing audio activity: $_"
        return $false
    }
    finally {
        if ($waveIn) {
            $waveIn.StopRecording()
            $waveIn.Dispose()
        }
        if ($audioMeter) {
            $audioMeter.Dispose()
        }
    }
}

function Clear-OldLogs {
    param($retentionDays)
    try {
        $cutoffDate = (Get-Date).AddDays(-$retentionDays)
        Get-ChildItem $logPath -File | Where-Object { $_.LastWriteTime -lt $cutoffDate } | Remove-Item
    }
    catch {
        Write-Log "Error clearing old logs: $_"
    }
}

function Start-BluetoothMonitor {
    $config = Get-Config
    $lastActivity = Get-Date
    $reconnectAttempts = 0
    
    Write-Log "Starting Bluetooth monitor for device: $($config.DeviceName)"
    
    while ($true) {
        try {
            # Check config updates
            $configLastWrite = (Get-Item $configPath).LastWriteTime
            if ($configLastWrite -gt $config.LastConfigCheck) {
                $config = Get-Config
                $config.LastConfigCheck = Get-Date
                Write-Log "Configuration reloaded"
            }

            # Clean old logs
            Clear-OldLogs -retentionDays $config.LogRetentionDays

            $device = Get-BluetoothDevice -deviceName $config.DeviceName
            if ($device) {
                if ($device.ConnectionStatus -eq "Connected") {
                    if (Test-AudioActivity -threshold $config.AudioThreshold) {
                        $lastActivity = Get-Date
                        $reconnectAttempts = 0
                    }
                    elseif (((Get-Date) - $lastActivity).TotalSeconds -gt $config.IdleTimeout) {
                        Write-Log "Disconnecting due to inactivity"
                        $device.Disconnect()
                    }
                }
                elseif ($config.AutoReconnect -and (Test-AudioActivity -threshold $config.AudioThreshold)) {
                    if ($reconnectAttempts -lt $config.ReconnectAttempts) {
                        Write-Log "Attempting to reconnect (Attempt $($reconnectAttempts + 1))"
                        $device.Connect()
                        $reconnectAttempts++
                        Start-Sleep -Seconds $config.ReconnectDelay
                    }
                    else {
                        Write-Log "Max reconnection attempts reached"
                        Start-Sleep -Seconds ($config.CheckInterval * 2)
                        $reconnectAttempts = 0
                    }
                }
            }
        }
        catch {
            Write-Log "Error in main loop: $_"
        }
        Start-Sleep -Seconds $config.CheckInterval
    }
}

# Hide PowerShell window
try {
    $windowCode = '[DllImport("user32.dll")] public static extern bool ShowWindow(int handle, int state);'
    add-type -name win -member $windowCode -namespace native
    [native.win]::ShowWindow(([System.Diagnostics.Process]::GetCurrentProcess() | Get-Process).MainWindowHandle, 0)
}
catch {
    Write-Log "Error hiding window: $_"
}

# Start monitoring with error handling
try {
    Start-BluetoothMonitor
}
catch {
    Write-Log "Fatal error in monitor: $_"
    exit 1
}
