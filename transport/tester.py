"""
Przy uruchamianiu programu należy wykorzystywać polecenie /usr/bin/time -v.
Umożliwi to pomiar czasu i zajętość pamięci (pola Elapsed time i Maximum resident setsize).

3 pkt. Uruchomić program do pobrania ok. 15 000 bajtów, gdzie liczba bajtów nie jest
wielokrotnością 1 000. Na takich samych danych uruchomić program
transport-client-slow; niech t będzie czasem jego działania. Program studenta
otrzymuje punkty, jeśli jego czas działania jest nie większy niż 4 · t + 5 sek, zajętość
pamięci nie większa niż 5 MB, a pliki generowane przez oba programy są identyczne.

1 pkt. Uruchomić program do pobrania ok. 15 000 bajtów. Zatrzymać go w trakcie wykonywania;
sprawdzić Wiresharkiem jaki jest jego port źródłowy.
Następnie poleceniem nc wysłać do tego portu źródłowego datagram zawierający śmieci.
Wznowić działanie programu i sprawdzić, czy generowany jest poprawny plik.

3 pkt. Jak w pierwszym punkcie, ale pobieramy ok. 1 000 000 bajtów i porównujemy czas
z czasem działania programu transport-client-fast; niech t będzie czasem jego
działania. Czas działania programu studenta nie powinien być większy niż 4·t+5 sek.

3 pkt. Jak w poprzednim punkcie, ale pobieramy ok. 9 000 000 bajtów.
"""

from __future__ import annotations

import argparse
import os
import subprocess
from enum import Enum, auto

from pathlib import Path
from typing import NamedTuple, Optional, Iterator

DEBUG = False

MAX_MEM_FOOTPRINT_KB = 5 * 1024

DIFF = "diff %s %s"


class ExecTime(NamedTuple):
    seconds: int
    milliseconds: float

    @property
    def total(self) -> float:
        return self.seconds + self.milliseconds / 1000.0

    def __str__(self) -> str:
        return f"{self.seconds}.{self.milliseconds} s."


class PerformanceStat:
    @staticmethod
    def is_perf_line(line: str) -> bool:
        return "Maximum" in line or "Elapsed" in line

    def __init__(self, resident_mem_footprint: int, exec_time: ExecTime):
        self.resident_mem_footprint = resident_mem_footprint
        self.exec_time = exec_time

    @classmethod
    def from_lines(cls, lines: Iterator[str]) -> PerformanceStat:
        """Assumed line formats:
        Time: Elapsed (wall clock) time (h:mm:ss or m:ss): 0:06.25
        Mem:  Maximum resident set size (kbytes): 1900
        """
        [seconds, milliseconds] = next(lines).split(" ")[-1].split(":")[:2]
        mem = int(next(lines).split(" ")[-1])
        return PerformanceStat(mem, ExecTime(int(seconds), float(milliseconds)))

    def __str__(self) -> str:
        return f"Max memory use : {self.resident_mem_footprint} KB\n" \
               f"Elapsed time   : {self.exec_time}"


class Profiler:
    PROFILER_COMMAND__ = r"/usr/bin/time -v %s"

    def __init__(self, client: Downloader):
        self.client_ref = client

    def profile(self) -> PerformanceStat:
        profile = Profiler.PROFILER_COMMAND__ % self.client_ref.command()
        proc = subprocess.Popen(profile.split(" "), stderr=subprocess.PIPE, stdout=subprocess.DEVNULL)
        stderr = proc.communicate()[1]
        stats = PerformanceStat.from_lines(filter(PerformanceStat.is_perf_line, stderr.decode("utf-8").splitlines()))
        return stats


class TestType(Enum):
    TEMPLATE = auto()
    SOLUTION = auto()

    def __str__(self) -> str:
        return self.name.lower()


class ServerConfig(NamedTuple):
    ip: str
    port: int
    
    def __str__(self) -> str:
        return f"{self.ip} {self.port}"


class TestConfig:
    TEST_CASES__: list[int] = [1, 3, 4] if not DEBUG else [3]

    TEST_LENGTHS__: dict[int, int] = {
        1: 15034,
        3: 1_000_000,
        4: 9_000_000,
    }
    
    TEMPLATE_BIN_FILE__: dict[int, Path] = {
        1: Path("transport-client-slow"),
        3: Path("transport-client-fast"),
        4: Path("transport-client-fast"),
    }

    def __init__(self, test_case: int, test_type: TestType):
        self.test_case = test_case
        self.test_type = test_type

    @staticmethod
    def get_client(binary_folder: Path, test_case: int) -> Path:
        return binary_folder / TestConfig.TEMPLATE_BIN_FILE__[test_case]

    @staticmethod
    def test_cases() -> Iterator[int]:
        return iter(TestConfig.TEST_CASES__)

    @property
    def cmp_file(self) -> Path:
        return Path(f"{self.test_type}-{self.test_case}")

    @property
    def length(self) -> int:
        return TestConfig.TEST_LENGTHS__[self.test_case]

    def __str__(self) -> str:
        return f"Test {self.test_case} :: Downloading {self.length} bytes"


class Downloader:
    
    SERVER_CONFIG__: Optional[ServerConfig] = None
    
    @staticmethod
    def set_server_config(server_config: ServerConfig):
        Downloader.SERVER_CONFIG__ = server_config
    
    def __init__(self, binary: Path, test_config: TestConfig):
        if Downloader.SERVER_CONFIG__ is None:
            raise ValueError("server configuration is not set")
        self.binary = binary
        self.test_config = test_config

    @classmethod
    def template(cls, binary_folder: Path, test_case: int) -> Downloader:
        return cls(TestConfig.get_client(binary_folder, test_case), TestConfig(test_case, TestType.TEMPLATE))

    @classmethod
    def solution(cls, binary: Path, test_case: int) -> Downloader:
        return cls(binary, TestConfig(test_case, TestType.SOLUTION))

    def download(self):
        os.system(self.command())

    def command(self) -> str:
        return f"{self.binary} {Downloader.SERVER_CONFIG__} {self.test_config.cmp_file} {self.test_config.length}"

    def __str__(self) -> str:
        return self.command()


def cmp_stats(temp: PerformanceStat, solution: PerformanceStat) -> (bool, bool):
    return solution.exec_time.total < temp.exec_time.total * 4.0 + 5.0, solution.resident_mem_footprint < MAX_MEM_FOOTPRINT_KB


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("server_address", help="Ipv4 address of the server")
    parser.add_argument("server_port", help="server's port")
    parser.add_argument("template_path", type=Path, help="Path to directory containing template client binaries")
    parser.add_argument("solution_path", type=Path, help="Path to rust project implementation")
    namespace = parser.parse_args()

    # configure server connection
    Downloader.set_server_config(ServerConfig(namespace.server_address, namespace.server_port))

    template_path = namespace.template_path
    solution_path = namespace.solution_path

    for test_case in TestConfig.test_cases():
        template_profiler = Profiler(Downloader.template(template_path, test_case))
        solution_profiler = Profiler(Downloader.solution(solution_path, test_case))
        print(solution_profiler.client_ref.test_config, end='\n\n')
        template_stats = template_profiler.profile()
        solution_stats = solution_profiler.profile()
        time_ok, mem_ok = cmp_stats(template_stats, solution_stats)
        print("Template results:")
        print("\t" + "\t".join(str(template_stats).splitlines(True)))
        print(f"User results:")
        print("\t" + "\t".join(str(solution_stats).splitlines(True)), end='\n\n')
        print(f"Mem : {'OK' if mem_ok else f'TOO BIG. Limit exceeded by {solution_stats.resident_mem_footprint - MAX_MEM_FOOTPRINT_KB}'}")
        print(f"Time: {'OK' if time_ok else 'TOO SLOW'}")


if __name__ == '__main__':
    main()
