import os
import urllib.request
import urllib.error
import re

class Registry:
    BASE_URL = "https://raw.githubusercontent.com/merchantmoh-debug/ark-packages/main/{name}/lib.ark"

    def install(self, name: str, lib_dir: str):
        if not re.match(r"^[a-zA-Z0-9_-]+$", name):
            raise ValueError(f"Invalid package name: '{name}'. Only alphanumeric, underscores, and hyphens allowed.")

        target_path = os.path.join(lib_dir, name, "lib.ark")
        url = self.BASE_URL.format(name=name)

        if not os.path.exists(os.path.dirname(target_path)):
            os.makedirs(os.path.dirname(target_path))

        print(f"Downloading {name} from {url}...")
        try:
            with urllib.request.urlopen(url) as response:
                content = response.read()
                with open(target_path, "wb") as f:
                    f.write(content)
            print(f"Installed {name} to {target_path}")
        except urllib.error.HTTPError as e:
            # Clean up empty directory if we created it
            dirname = os.path.dirname(target_path)
            if os.path.exists(dirname) and not os.listdir(dirname):
                os.rmdir(dirname)

            if e.code == 404:
                raise ValueError(f"Package '{name}' not found at registry.")
            else:
                raise RuntimeError(f"Error downloading package: {e}")
        except Exception as e:
            # Clean up empty directory if we created it
            dirname = os.path.dirname(target_path)
            if os.path.exists(dirname) and not os.listdir(dirname):
                os.rmdir(dirname)
            raise

    def search(self, query: str):
        if not re.match(r"^[a-zA-Z0-9_-]+$", query):
             print(f"Invalid query: '{query}'. Only alphanumeric, underscores, and hyphens allowed.")
             return False

        url = self.BASE_URL.format(name=query)
        try:
            # Check existence via HEAD request
            request = urllib.request.Request(url, method="HEAD")
            with urllib.request.urlopen(request) as response:
                if response.status == 200:
                    print(f"Package '{query}' found.")
                    return True
        except urllib.error.HTTPError as e:
            if e.code == 404:
                print(f"Package '{query}' not found.")
                return False
            else:
                print(f"Error searching {query}: {e}")
                return False
        except Exception as e:
            print(f"Error searching {query}: {e}")
            return False
