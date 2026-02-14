use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn escape_single_quotes(s: &str) -> String {
    s.replace("'", "''")
}

/// Open a folder using PowerShell -> Start-Process (hidden)
/// Returns Err(String) on failure.
pub fn open_folder(path: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let p = escape_single_quotes(path);
        // Use explorer to open folders so the behavior is consistent
        let ps_cmd = format!("Start-Process -FilePath 'explorer' -ArgumentList '{}'", p);

        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(ps_cmd);
        // prevent flashing console window
        cmd.creation_flags(CREATE_NO_WINDOW);

        let status = cmd.spawn().map_err(|e| e.to_string())?.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("process exited with {}", status))
        }
    }

    #[cfg(not(windows))]
    {
        // On non-Windows, fall back to system openers
        let status = if cfg!(target_os = "macos") {
            Command::new("open").arg(path).status()
        } else {
            Command::new("xdg-open").arg(path).status()
        }
        .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("process exited with {}", status))
        }
    }
}

/// Open a file using PowerShell -> Start-Process (hidden)
pub fn open_file(path: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let p = escape_single_quotes(path);
        let ps_cmd = format!("Start-Process -FilePath '{}'", p);

        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(ps_cmd);
        cmd.creation_flags(CREATE_NO_WINDOW);

        let status = cmd.spawn().map_err(|e| e.to_string())?.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("process exited with {}", status))
        }
    }

    #[cfg(not(windows))]
    {
        let status = if cfg!(target_os = "macos") {
            Command::new("open").arg(path).status()
        } else {
            Command::new("xdg-open").arg(path).status()
        }
        .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("process exited with {}", status))
        }
    }
}

/// Copy file using PowerShell (Hidden) to bypass some permission issues or just use native shell
pub fn copy_file_powershell(src: &str, dest: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let s = escape_single_quotes(src);
        let d = escape_single_quotes(dest);
        
        // Ensure directory exists then copy
        // $d is the full file path. We need to create the parent directory.
        // PowerShell: New-Item -ItemType Directory -Force -Path (Split-Path -Path 'dest' -Parent); Copy-Item -Path 'src' -Destination 'dest' -Force
        
        // Note: We use Split-Path to get parent dir from the destination file path
        let ps_cmd = format!(
            "New-Item -ItemType Directory -Force -Path (Split-Path -Path '{}' -Parent); Copy-Item -Path '{}' -Destination '{}' -Force", 
            d, s, d
        );

        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(ps_cmd);
        // Hide window
        cmd.creation_flags(CREATE_NO_WINDOW);

        let status = cmd.spawn().map_err(|e| e.to_string())?.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("Copy process exited with {}", status))
        }
    }
    #[cfg(not(windows))]
    {
         // Fallback to standard FS for non-windows (should retain same permissions usually)
         use std::fs;
         use std::path::Path;
         
         if let Some(parent) = Path::new(dest).parent() {
             fs::create_dir_all(parent).map_err(|e| e.to_string())?;
         }
         fs::copy(src, dest).map_err(|e| e.to_string())?;
         Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_open_folder() {
        let _ = open_folder("C:/");
    }
}
