# colorize
Create coherent terminal colors based on OKLAB, APCA and WCAG

`-b` | `--background` Background color. Currently `000000` is recommended.
`-s` | `--saturation` `[0-100]` 0: faint, 100: colorful
`-l` | `--lightness` `[0-100]` 0: dark, 100: light
`-o` | `--offset` `[0-359]` The hue point where the first color starts
`-c` | `--count` `[1-inf]` Amount of colors to output. `6` is recommended for terminals and text editors.
`-r` | `--random` Create random colorschemes (currently hardcoded to conservative contrast values for minimum requirements)
`-a` | `--analyze` Analyze popular colorschemes like Gruvbox, Dracula against OKHSL coherence and APCA/WCAG contrast.

- For terminals you may need additional colors such as a main, white foreground color; a darker white for comments and terminal autosuggestions.
- For text editors you can either duplicate the same colors for certain categories or choose slighhtly different versions. Refer to [BASE16 styling guide](https://github.com/chriskempson/base16/blob/main/styling.md)
