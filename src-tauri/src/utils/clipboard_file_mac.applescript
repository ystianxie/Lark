on run argv
    -- 创建一个列表来存储文件路径
    set theFiles to {}
    repeat with i in argv
        -- 将每个路径转换为 Finder 文件对象
        set end of theFiles to POSIX file i
    end repeat

    -- 使用 Finder 进行剪贴板操作
    tell application "Finder"
        set the clipboard to theFiles
    end tell
end run
