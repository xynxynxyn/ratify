#!/bin/python3
import sys
import time
import os
import subprocess


# return process info and execution time
def time_cmd(cmd):
    start_time = time.perf_counter()

    process = subprocess.run(cmd,
                             stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE,
                             universal_newlines=True)

    end_time = time.perf_counter()
    return process, end_time - start_time


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: ./run_tests.py [TEST_DIR]")
        exit(1)

    # build the executable
    print("-- building executable")
    subprocess.run(["cargo", "build", "--release"])

    test_dir = sys.argv[1]
    print(f"-- test directory '{test_dir}'")

    for f in os.listdir(test_dir):
        path = os.path.join(test_dir, f)
        if os.path.isfile(path):
            print(f"-- '{f}'")
            _, cert_time = time_cmd(
                ["glucose", "-certified", "-certified-output=cert.tmp", path])
            print(f"    created certificate in {cert_time}s")
            drat_process, drat_time = time_cmd(["drat-trim", path, "cert.tmp"])
            ratify_process, ratify_time = time_cmd(
                ["target/release/ratify", path, "cert.tmp", "-m"])
            if (ratify_process.returncode == 0
                    and drat_process.returncode == 0):
                print("    refutation successfully validated")
                print(f"    ratify took {ratify_time}s")
                print(f"    drat-trim took {drat_time}s")
                print("    OK")
            elif (ratify_process.returncode != 0
                  and drat_process.returncode != 0):
                print("    refutation successfully rejected")
                print(f"    ratify took {ratify_time}s")
                print(f"    drat-trim took {drat_time}s")
                print("    OK")
            else:
                print(
                    f"    ERR: drat-trim={drat_process.returncode} ratify={ratify_process.returncode}"
                )
    os.remove("cert.tmp")
