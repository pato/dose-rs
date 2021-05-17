# Dose-rs

This Rust app will use Doctolib to find vaccination appointments in France.

# Usage

Change the `centers` vector to match the vaccination centers that you care about.

The exit code will either be:
* 0 -> successfully found one or more vaccination slots
* 1 -> an unexpected error occurred
* 2 -> no vaccination slots were found

You can pass in `--verbose` or `-v` to have it print each center and place as
it checks them. Otherwise it will print info only if it finds an appointment.

<img width="1243" alt="image" src="https://user-images.githubusercontent.com/608083/118532649-7f39fd80-b747-11eb-801e-7c3d1573a5e8.png">


