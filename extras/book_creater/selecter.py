import re
import sys
import os
import random

def parse_log(log_content):
    """
    Parses the log content to extract test results for each theme.
    Returns a dictionary of themes, each containing a list of 'passed' and 'failed' tests.
    """
    test_suites = {}
    current_suite = None
    lines = log_content.splitlines()

    for line in lines:
        # Check for the start of a new test suite
        if line.startswith("Running test suite:"):
            match = re.search(r'./test_suites/sts/(.*)\.epd', line)
            if match:
                # Infer the theme from the filename
                theme = match.group(1)
                # Clean up theme names for a cleaner output
                cleaned_theme = theme.replace('STS(v2.2) ', '').replace('STS(v3.1) ', '').replace('STS: ', '').replace('STS(v4.0) ', '').replace('STS(v5.0) ', '').replace('STS(v6.0) ', '').strip()
                current_suite = cleaned_theme
                if current_suite not in test_suites:
                    test_suites[current_suite] = {'passes': [], 'failures': []}
            continue

        if not current_suite:
            continue

        # Check for PASS entries
        pass_match = re.search(r'\[\s*\x1b\[32mPASS\s*\x1b\[0m\]\s*ID:\s*(.*)\s*\|\s*S:\s*(\d+)/10\s*\|', line)
        if pass_match:
            test_id, score_str = pass_match.groups()
            score = int(score_str)
            test_suites[current_suite]['passes'].append({
                'id': test_id.strip(),
                'score': score,
                'theme': current_suite
            })
            continue

        # Check for FAIL entries
        fail_match = re.search(r'\[\s*\x1b\[31mFAIL\s*\x1b\[0m\]\s*ID:\s*(.*)\s*\|\s*S:\s*(\d+)/10\s*\|', line)
        if fail_match:
            test_id, score_str = fail_match.groups()
            score = int(score_str)
            test_suites[current_suite]['failures'].append({
                'id': test_id.strip(),
                'score': score,
                'theme': current_suite
            })
            continue
    return test_suites

def select_tests(test_suites, total_selections=60):
    """
    Selects test cases based on a strategy:
    - Prioritizes failures.
    - Includes a small number of passes.
    - Spreads selections evenly across themes.
    """
    selected_tests = []
    themes = list(test_suites.keys())
    num_themes = len(themes)
    tests_per_theme = total_selections // num_themes

    for theme in themes:
        theme_tests = test_suites[theme]
        num_failures_to_select = int(tests_per_theme * 0.7)
        num_passes_to_select = tests_per_theme - num_failures_to_select

        # Sort failures by score in ascending order (hardest fails first)
        failures = sorted(theme_tests['failures'], key=lambda x: x['score'])
        
        # Select failures (prioritize hard failures)
        selected_failures = []
        if failures:
            # Take a few hard fails
            hard_fails = [f for f in failures if f['score'] <= 2]
            selected_failures.extend(hard_fails[:num_failures_to_select // 2])
            
            # Add some near misses
            remaining_failures = [f for f in failures if f['score'] > 2]
            random.shuffle(remaining_failures)
            selected_failures.extend(remaining_failures[:num_failures_to_select - len(selected_failures)])
            selected_tests.extend(selected_failures)

        # Select passes
        passes = theme_tests['passes']
        if passes:
            selected_passes = random.sample(passes, min(num_passes_to_select, len(passes)))
            selected_tests.extend(selected_passes)

    # Shuffle the final list to mix up the themes in the output
    random.shuffle(selected_tests)

    # If the total number of selections is not exactly 60, adjust the list
    if len(selected_tests) > total_selections:
        selected_tests = selected_tests[:total_selections]
    
    return selected_tests

def read_epd_files(epd_directory):
    """
    Reads all EPD files in a directory and returns a list of all lines.
    """
    all_lines = []
    for filename in os.listdir(epd_directory):
        if filename.endswith('.epd'):
            filepath = os.path.join(epd_directory, filename)
            try:
                with open(filepath, 'r') as f:
                    all_lines.extend(f.readlines())
            except Exception as e:
                print(f"Error reading file {filepath}: {e}", file=sys.stderr)
    return all_lines

def find_test_lines(selected_tests, all_epd_lines):
    """
    Finds and prints the full lines for the selected test IDs from the in-memory EPD data.
    """
    found_lines = {}
    
    for test in selected_tests:
        test_id = test['id']
        for line in all_epd_lines:
            if test_id in line:
                found_lines[test_id] = line.strip()
                break
    
    # Print the found lines to stdout
    print("Found test lines for selected IDs:")
    for test_id in found_lines:
        print(found_lines[test_id])


def print_summary(selected_tests, all_parsed_data):
    """
    Analyzes and prints a summary of the selected test cases per theme.
    """
    print("\n--- Summary of Selections per Theme ---")
    
    theme_summary = {}
    for test in selected_tests:
        theme = test['theme']
        if theme not in theme_summary:
            theme_summary[theme] = {'passes': 0, 'fails': 0, 'total_score': 0, 'count': 0}
        
        if test['score'] == 10:
            theme_summary[theme]['passes'] += 1
        else:
            theme_summary[theme]['fails'] += 1
        
        theme_summary[theme]['total_score'] += test['score']
        theme_summary[theme]['count'] += 1

    for theme, data in theme_summary.items():
        avg_score = (data['total_score'] / data['count']) if data['count'] > 0 else 0
        print(f"Theme: {theme}")
        print(f"  - Passes/Fails: {data['passes']}/{data['fails']}")
        print(f"  - Average Score: {avg_score:.2f}/10\n")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python select_and_find_tests.py <directory_with_epd_files>")
        sys.exit(1)
    
    epd_dir = sys.argv[1]

    # Assume the log file is in the same directory as the script
    log_file_path = 'eval_test_stc_6threads.txt'
    
    if not os.path.exists(log_file_path):
        print(f"Error: Log file '{log_file_path}' not found.", file=sys.stderr)
        sys.exit(1)

    try:
        # Read the log file to get test selections
        with open(log_file_path, 'r') as f:
            log_content = f.read()
        
        parsed_data = parse_log(log_content)
        selected_data = select_tests(parsed_data, total_selections=60)
        
        # Read all EPD files into memory
        all_epd_lines = read_epd_files(epd_dir)
        
        # Find the full lines for the selected tests
        find_test_lines(selected_data, all_epd_lines)

        # Print the summary
        print_summary(selected_data, parsed_data)
    except Exception as e:
        print(f"An error occurred: {e}", file=sys.stderr)
        sys.exit(1)
