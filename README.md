# Pokésearch

A simple command-line tool for searching information regarding Pokémon game mechanics.

## Usage

To run Pokésearch, you can either install from source, or check the
[latest release](https://github.com/Jaxydog/pokesearch/releases) for a compiled binary.

```
cargo install --git https://github.com/Jaxydog/pokesearch.git
pokesearch --help
```

The application will cache its query results in a directory that can be configured using the `--cache-dir` argument.
By default, this directory will be `$CWD/.cache`.

Pokésearch comes with the following sub-commands:

- `pokesearch pokemon <name>` - List data about a specific Pokémon.
- `pokesearch ability <name>` - List an ability's description.
- `pokesearch move <name>` - List data about a specific move.
- `pokesearch item <name>` - List data about a specific item.
- `pokesearch type <name...>` - Display a type match-up for the given type name(s).

## License

Pokésearch is licensed under the GNU Affero General Public License version 3, or (at your option) any later version.
You should have received a copy of the GNU Affero General Public License along with rs, found in [LICENSE](./LICENSE.md).
If not, see \<[https://www.gnu.org/licenses/](https://www.gnu.org/licenses/)>.
