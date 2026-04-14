```bash
  python3 -m venv .venv
  source .venv/bin/activate
  pip install pillow
  python tools/font_to_c_font.py assets/fonts/Ubuntu-Regular.ttf 12 components/papers3_display/ubuntu_font_12.h ubuntu_12
```