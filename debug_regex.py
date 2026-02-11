
import re

regex = r'"(\\"|[^"])*"'
text = """
if 1 == 1 {
    print("True")
} else {
    print("False")
}
"""

matches = re.finditer(regex, text)
for match in matches:
    print(f"Match: {match.group(0)!r}")
