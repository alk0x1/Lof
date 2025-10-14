#!/usr/bin/env python3
"""
Lof Verification System
Verifies Lof R1CS compiler against Circom reference implementation

Usage:
    python3 verify.py multiply           # Verify single circuit
    python3 verify.py --all              # Verify all circuits
    python3 verify.py --clean            # Clean outputs
    python3 verify.py --list             # List available circuits
"""

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Dict, Tuple


class VerificationError(Exception):
    pass


class Verifier:
    def __init__(self, root_dir: Path):
        self.root_dir = root_dir
        self.circuits_dir = root_dir / "circuits"
        self.outputs_dir = root_dir / "outputs"
        self.test_cases_dir = root_dir / "test_cases"

    def find_circuit(self, circuit_name: str) -> Tuple[Path, Path]:
        """Find .lof and .circom files for a circuit"""
        for category_dir in self.circuits_dir.iterdir():
            if not category_dir.is_dir():
                continue

            lof_file = category_dir / f"{circuit_name}.lof"
            circom_file = category_dir / f"{circuit_name}.circom"

            if lof_file.exists() and circom_file.exists():
                return lof_file, circom_file

        raise VerificationError(f"Circuit '{circuit_name}' not found")

    def find_test_cases(self, circuit_name: str) -> Path:
        """Find test cases file for a circuit"""
        for category_dir in self.test_cases_dir.iterdir():
            if not category_dir.is_dir():
                continue

            test_file = category_dir / f"{circuit_name}_tests.json"
            if test_file.exists():
                return test_file

        raise VerificationError(f"Test cases not found for '{circuit_name}'")

    def compile_both(self, circuit_name: str) -> bool:
        """Compile circuit in both Lof and Circom"""
        print(f"\n[1/3] Compiling {circuit_name}...")

        lof_file, circom_file = self.find_circuit(circuit_name)
        category_dir = lof_file.parent

        # Create output directories
        circom_out = self.outputs_dir / "circom"
        lof_out = self.outputs_dir / "lof"
        circom_out.mkdir(parents=True, exist_ok=True)
        lof_out.mkdir(parents=True, exist_ok=True)

        # Compile Circom
        print(f"  Circom: {circom_file.name}")
        result = subprocess.run(
            ["circom", circom_file.name, "--r1cs", "--wasm", "--sym", "--output", str(circom_out)],
            cwd=category_dir,
            capture_output=True,
            text=True
        )
        if result.returncode != 0:
            print(f"  FAILED: {result.stderr}")
            return False
        print("  SUCCESS")

        # Compile Lof
        print(f"  Lof: {lof_file.name}")
        result = subprocess.run(
            ["lof", "compile", lof_file.name],
            cwd=category_dir,
            capture_output=True,
            text=True
        )
        if result.returncode != 0:
            print(f"  FAILED: {result.stderr}")
            return False

        # Move R1CS file
        r1cs_file = category_dir / f"{circuit_name}.r1cs"
        build_r1cs = category_dir / "build" / f"{circuit_name}.r1cs"

        if r1cs_file.exists():
            shutil.move(str(r1cs_file), str(lof_out / f"{circuit_name}.r1cs"))
        elif build_r1cs.exists():
            shutil.move(str(build_r1cs), str(lof_out / f"{circuit_name}.r1cs"))
        else:
            print("  FAILED: R1CS file not found")
            return False

        # Cleanup build directories
        for cleanup_dir in ["build", "keys", "inputs", "proofs"]:
            cleanup_path = category_dir / cleanup_dir
            if cleanup_path.exists():
                shutil.rmtree(cleanup_path)

        print("  SUCCESS")
        return True

    def generate_witnesses(self, circuit_name: str) -> bool:
        """Generate witnesses for all test cases"""
        print(f"\n[2/3] Generating witnesses...")

        test_file = self.find_test_cases(circuit_name)

        with open(test_file) as f:
            test_data = json.load(f)

        # Create output directories
        circom_witnesses = self.outputs_dir / "circom" / "witnesses"
        lof_witnesses = self.outputs_dir / "lof" / "witnesses"
        circom_witnesses.mkdir(parents=True, exist_ok=True)
        lof_witnesses.mkdir(parents=True, exist_ok=True)

        test_vectors = test_data.get("test_vectors", [])
        print(f"  Found {len(test_vectors)} test cases")

        for i, test_vector in enumerate(test_vectors):
            test_name = test_vector.get("name", f"test_{i}")
            print(f"  [{i+1}/{len(test_vectors)}] {test_name}")

            # Write input file
            input_file = self.outputs_dir / f"test_input_{i}.json"
            with open(input_file, 'w') as f:
                json.dump(test_vector["inputs"], f)

            # Generate Circom witness
            circom_wasm = self.outputs_dir / "circom" / f"{circuit_name}_js" / f"{circuit_name}.wasm"
            circom_gen = self.outputs_dir / "circom" / f"{circuit_name}_js" / "generate_witness.js"
            circom_wtns = circom_witnesses / f"test_input_{i}.wtns"

            if not circom_wasm.exists():
                print(f"    FAILED: Circom WASM not found")
                return False

            result = subprocess.run(
                ["node", str(circom_gen), str(circom_wasm), str(input_file), str(circom_wtns)],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                print(f"    FAILED: Circom witness - {result.stderr}")
                return False

            # Convert witness to JSON
            circom_json = circom_witnesses / f"test_input_{i}.json"
            result = subprocess.run(
                ["snarkjs", "wtns", "export", "json", str(circom_wtns), str(circom_json)],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                print(f"    WARNING: snarkjs not found, using binary witness")

            # Generate Lof witness using lofit
            lof_r1cs = self.outputs_dir / "lof" / f"{circuit_name}.r1cs"
            temp_dir = self.outputs_dir / f"temp_lofit_{i}"
            temp_dir.mkdir(exist_ok=True)

            try:
                # Copy files to temp directory
                shutil.copy(str(lof_r1cs), str(temp_dir / f"{circuit_name}.r1cs"))
                shutil.copy(str(input_file), str(temp_dir / "inputs.json"))

                # Run lofit setup
                result = subprocess.run(
                    ["cargo", "run", "--release", "--bin", "lofit", "--", "setup",
                     "--input", f"{circuit_name}.r1cs",
                     "--proving-key", "pk.bin",
                     "--verification-key", "vk.bin"],
                    cwd=temp_dir,
                    capture_output=True,
                    text=True
                )
                if result.returncode != 0:
                    print(f"    FAILED: lofit setup - {result.stderr}")
                    return False

                # Run lofit prove
                result = subprocess.run(
                    ["cargo", "run", "--release", "--bin", "lofit", "--", "prove",
                     "--input", f"{circuit_name}.r1cs",
                     "--proving-key", "pk.bin",
                     "--public-inputs", "inputs.json",
                     "--output", "proof.bin"],
                    cwd=temp_dir,
                    capture_output=True,
                    text=True
                )
                if result.returncode != 0:
                    print(f"    FAILED: lofit prove - {result.stderr}")
                    return False

                # Convert witness format - try both locations
                full_witness_locations = [
                    temp_dir / "full_witness.json",  # Current location (when output has no parent dir)
                    temp_dir / "proofs" / "full_witness.json"  # Expected location
                ]

                full_witness = None
                for location in full_witness_locations:
                    if location.exists():
                        full_witness = location
                        break

                if full_witness:
                    with open(full_witness) as f:
                        witness_data = json.load(f)
                    witness_array = list(witness_data.values())

                    lof_json = lof_witnesses / f"test_input_{i}.json"
                    with open(lof_json, 'w') as f:
                        json.dump(witness_array, f)
                else:
                    print(f"    FAILED: Lof witness not generated in {temp_dir}")
                    return False

            finally:
                # Cleanup temp directory
                shutil.rmtree(temp_dir, ignore_errors=True)

            print(f"    SUCCESS")

        return True

    def compare_results(self, circuit_name: str) -> bool:
        """Compare Lof and Circom witness results"""
        print(f"\n[3/3] Comparing results...")

        test_file = self.find_test_cases(circuit_name)

        with open(test_file) as f:
            test_data = json.load(f)

        # Load symbol mapping for Circom
        sym_file = self.outputs_dir / "circom" / f"{circuit_name}.sym"
        circom_mapping = self._parse_sym_file(sym_file) if sym_file.exists() else {}

        test_vectors = test_data.get("test_vectors", [])
        passed = 0
        failed = 0

        for i, test_vector in enumerate(test_vectors):
            test_name = test_vector.get("name", f"test_{i}")
            expected = test_vector.get("expected_outputs", {})

            # Load witnesses
            circom_file = self.outputs_dir / "circom" / "witnesses" / f"test_input_{i}.json"
            lof_file = self.outputs_dir / "lof" / "witnesses" / f"test_input_{i}.json"

            if not circom_file.exists():
                print(f"  [{i+1}] {test_name}: FAILED - Circom witness not found")
                failed += 1
                continue

            if not lof_file.exists():
                print(f"  [{i+1}] {test_name}: FAILED - Lof witness not found")
                failed += 1
                continue

            with open(circom_file) as f:
                circom_witness = json.load(f)

            with open(lof_file) as f:
                lof_witness = json.load(f)

            # Extract outputs
            circom_outputs = self._extract_outputs_from_witness(circom_witness, expected, circom_mapping)
            lof_outputs = self._extract_outputs_from_witness(lof_witness, expected, {})

            # Compare
            all_match = True
            for output_name, expected_value in expected.items():
                circom_value = str(circom_outputs.get(output_name, "MISSING"))
                lof_value = str(lof_outputs.get(output_name, "MISSING"))
                expected_str = str(expected_value)

                if circom_value != expected_str or lof_value != expected_str or circom_value != lof_value:
                    all_match = False
                    print(f"  [{i+1}] {test_name}: FAILED")
                    print(f"      {output_name}: expected={expected_str}, circom={circom_value}, lof={lof_value}")

            if all_match:
                print(f"  [{i+1}] {test_name}: PASS")
                passed += 1
            else:
                failed += 1

        print(f"\n  Summary: {passed} passed, {failed} failed")
        return failed == 0

    def _parse_sym_file(self, sym_file: Path) -> Dict[str, int]:
        """Parse Circom .sym file to get signal index mapping"""
        mapping = {}
        try:
            with open(sym_file) as f:
                for line in f:
                    parts = line.strip().split(',')
                    if len(parts) >= 4:
                        witness_idx = int(parts[0])
                        signal_name = parts[3].replace('main.', '')
                        mapping[signal_name] = witness_idx
        except Exception:
            pass
        return mapping

    def _extract_outputs_from_witness(self, witness, expected_outputs: Dict, signal_mapping: Dict) -> Dict:
        """Extract output values from witness using signal mapping"""
        result = {}

        if isinstance(witness, dict):
            # Dictionary witness format (from lofit potentially)
            for output_name in expected_outputs.keys():
                if output_name in witness:
                    result[output_name] = witness[output_name]
        elif isinstance(witness, list):
            # Array witness format (from circom)
            if signal_mapping:
                # Use signal mapping from .sym file
                for output_name in expected_outputs.keys():
                    if output_name in signal_mapping:
                        idx = signal_mapping[output_name]
                        if idx < len(witness):
                            result[output_name] = witness[idx]
            else:
                # Fallback: try to guess based on witness structure
                # For simple circuits with single output, skip witness[0] (constant 1)
                if len(expected_outputs) == 1:
                    output_name = list(expected_outputs.keys())[0]
                    if len(witness) > 1:
                        result[output_name] = witness[1]
                elif len(expected_outputs) == 3 and len(witness) >= 4:
                    # Multi-output case like multi_witness
                    output_names = sorted(expected_outputs.keys())
                    for i, name in enumerate(output_names):
                        if i + 1 < len(witness):
                            result[name] = witness[i + 1]

        return result

    def verify_circuit(self, circuit_name: str) -> bool:
        """Run complete verification pipeline"""
        print("=" * 60)
        print(f"VERIFICATION: {circuit_name}")
        print("=" * 60)

        try:
            if not self.compile_both(circuit_name):
                print("\nFAILED: Compilation")
                return False

            if not self.generate_witnesses(circuit_name):
                print("\nFAILED: Witness generation")
                return False

            if not self.compare_results(circuit_name):
                print("\nFAILED: Comparison")
                return False

            print("\n" + "=" * 60)
            print(f"PASSED: {circuit_name}")
            print("=" * 60)
            return True

        except VerificationError as e:
            print(f"\nERROR: {e}")
            return False
        except KeyboardInterrupt:
            print("\n\nInterrupted")
            return False

    def verify_all(self) -> Dict[str, bool]:
        """Verify all circuits"""
        print("=" * 60)
        print("VERIFYING ALL CIRCUITS")
        print("=" * 60)

        results = {}

        for category_dir in sorted(self.circuits_dir.iterdir()):
            if not category_dir.is_dir():
                continue

            category = category_dir.name
            print(f"\nCategory: {category}")
            print("-" * 60)

            for lof_file in sorted(category_dir.glob("*.lof")):
                circuit_name = lof_file.stem

                try:
                    success = self.verify_circuit(circuit_name)
                    results[f"{category}/{circuit_name}"] = success
                except Exception as e:
                    print(f"ERROR: {e}")
                    results[f"{category}/{circuit_name}"] = False

        # Summary
        print("\n" + "=" * 60)
        print("SUMMARY")
        print("=" * 60)

        total = len(results)
        passed = sum(1 for s in results.values() if s)
        failed = total - passed

        for circuit, success in results.items():
            status = "PASS" if success else "FAIL"
            print(f"  {status}: {circuit}")

        print(f"\nTotal: {total}, Passed: {passed}, Failed: {failed}")

        return results

    def clean(self):
        """Clean all output files"""
        print("Cleaning verification outputs...")

        # Clean circom outputs
        circom_out = self.outputs_dir / "circom"
        if circom_out.exists():
            for item in circom_out.iterdir():
                if item.is_dir():
                    shutil.rmtree(item)
                else:
                    item.unlink()

        # Clean lof outputs
        lof_out = self.outputs_dir / "lof"
        if lof_out.exists():
            for item in lof_out.iterdir():
                if item.is_dir():
                    shutil.rmtree(item)
                else:
                    item.unlink()

        # Clean test input files
        for f in self.outputs_dir.glob("test_input_*.json"):
            f.unlink()

        # Clean circuit build directories
        for cleanup_dir in ["build", "keys", "inputs", "proofs"]:
            for d in self.circuits_dir.rglob(cleanup_dir):
                if d.is_dir():
                    shutil.rmtree(d)

        print("Cleaned")

    def list_circuits(self):
        """List all available circuits"""
        for lof_file in sorted(self.circuits_dir.rglob("*.lof")):
            print(lof_file.stem)


def main():
    parser = argparse.ArgumentParser(
        description="Verify Lof R1CS compiler against Circom"
    )
    parser.add_argument(
        "circuit",
        nargs="?",
        help="Circuit name to verify"
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Verify all circuits"
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Clean output files"
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List all circuits"
    )

    args = parser.parse_args()

    root_dir = Path(__file__).parent
    verifier = Verifier(root_dir)

    if args.clean:
        verifier.clean()
        return 0

    if args.list:
        verifier.list_circuits()
        return 0

    if args.all:
        results = verifier.verify_all()
        failed = sum(1 for s in results.values() if not s)
        return 1 if failed > 0 else 0

    if args.circuit:
        success = verifier.verify_circuit(args.circuit)
        return 0 if success else 1

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
