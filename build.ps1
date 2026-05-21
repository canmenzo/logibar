# Builds logibar.exe — shows "logibar" in Windows tray settings instead of "Python"
# Run once: python -m pip install pyinstaller

python -m pip install pyinstaller --quiet
pyinstaller --onefile --windowed --name logibar --add-data "ghub.py;." main.py
Write-Host "Done. Executable: dist\logibar.exe"
