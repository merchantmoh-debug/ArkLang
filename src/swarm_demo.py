"""
Swarm Demo - Demonstration of multi-agent collaboration.

Run this script to see the Antigravity Swarm in action.

Usage:
    python -m src.swarm_demo
"""

from src.swarm import SwarmOrchestrator


def main():
    """Run swarm demonstration with example tasks."""
    
    print("╔════════════════════════════════════════════════════════════════════╗")
    print("║         🪐 Antigravity Multi-Agent Swarm Demonstration            ║")
    print("╚════════════════════════════════════════════════════════════════════╝\n")
    
    # Initialize the swarm
    swarm = SwarmOrchestrator()
    
    # Demo tasks
    demo_tasks = [
        {
            "name": "Code Generation",
            "task": "Create a Python function to calculate the Fibonacci sequence"
        },
        {
            "name": "Code Review",
            "task": "Review this code for security issues: def login(user, password): exec(f'authenticate({user}, {password})')"
        },
        {
            "name": "Research Task",
            "task": "Research the best practices for implementing JWT authentication in Python"
        },
        {
            "name": "Multi-Agent Collaboration",
            "task": "Build a simple calculator function and review it for code quality"
        }
    ]
    
    # Let user choose a demo or run all
    print("Available demonstrations:")
    for i, demo in enumerate(demo_tasks, 1):
        print(f"{i}. {demo['name']}: {demo['task']}")
    print(f"{len(demo_tasks) + 1}. Run all demonstrations")
    print("0. Enter custom task\n")
    
    try:
        choice = input("Select demo (0-5): ").strip()
        
        if choice == "0":
            # Custom task
            custom_task = input("\nEnter your task: ").strip()
            if custom_task:
                print("\n" + "=" * 70)
                result = swarm.execute(custom_task, verbose=True)
                print(f"\n📊 Final Result:\n{result}\n")
        elif choice.isdigit() and 1 <= int(choice) <= len(demo_tasks):
            # Run selected demo
            demo = demo_tasks[int(choice) - 1]
            print(f"\n Running: {demo['name']}\n")
            print("=" * 70)
            result = swarm.execute(demo['task'], verbose=True)
            print(f"\n📊 Final Result:\n{result}\n")
        elif choice == str(len(demo_tasks) + 1):
            # Run all demos
            for i, demo in enumerate(demo_tasks, 1):
                print(f"\n\n{'#' * 70}")
                print(f"Demo {i}/{len(demo_tasks)}: {demo['name']}")
                print('#' * 70)
                result = swarm.execute(demo['task'], verbose=True)
                print(f"\n📊 Final Result:\n{result}\n")
                
                if i < len(demo_tasks):
                    input("\nPress Enter to continue to next demo...")
                    swarm.reset()  # Reset for next demo
        else:
            print("Invalid choice. Running default demo...")
            result = swarm.execute(demo_tasks[0]['task'], verbose=True)
            print(f"\n📊 Final Result:\n{result}\n")
    
    except KeyboardInterrupt:
        print("\n\n👋 Demo interrupted. Goodbye!")
    except Exception as e:
        print(f"\n❌ Error: {e}")
    
    print("\n" + "=" * 70)
    print("✨ Demo complete! The swarm is ready for your tasks.")
    print("=" * 70)


if __name__ == "__main__":
    main()
