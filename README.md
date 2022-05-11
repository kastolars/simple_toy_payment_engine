# Simple Toy Payment Engine

Authored by Karol Stolarski

## Description

A simple toy payment engine, written in Rust.
A loosely structured guide follows describing thought processes and implementation.

## Loading the dataset

The `csv` crate provides a way to iterate over rows in the ingested file.
This foregoes the need to load the entire file into memory,
as an iterator will leverage a pointer to consecutive rows in the CSV.

This should cap the required memory consumption for reading a single record to ~14 bytes per loop iteration.

## Keeping state

`ClientAccountState` represents the in-memory state of a client's account.
Each object takes up 9 bytes for state data
and ~5 bytes of related transaction-specific metadata
(HashMaps may need more memory to store metadata about its entries other than keys and values).
So a representation of memory usage for state is roughly
`(num_clients * 9) + (num_transactions * 9)`, with a maximum memory requirement of ~36GB.

The value of keeping track of transaction metadata in a HashMap is leveraging `O(1)` lookup times
for transactions when needing to perform things like disputes. This is a tradeoff of speed for memory,
but it is also simple to implement.

## Synchronous Processing

This application processes transactions synchronously.
This is paramount as to avoid abusing the system.

## Precision

Values in the application are always cast to 4 decimal place precision.

## Coverage

I have constructed unit tests for processing each type of transaction.
Running them will cover several possible scenarios and outcomes including edge cases.

## Building the application

```
cargo build
```

## Running the application

```
cargo run transactions.csv > accounts.csv
```

## Scoring Criteria

### Basics

* Does your application build?
    * Yes, it compiles with `cargo build`
* Does it read and write data in the way we'd like it to?
    * Yes, the application reads data from the first argument to the program and outputs it to stdout.
      Providing `> output.csv` arguments should pipe std output to the output file argument.
* Is it properly formatted?
    * Output matches the directions, ie. columns are `client,available,held,total,locked`

### Completeness

* Do you handle all the cases, including disputes, resolutions, and chargebacks?
    * All 5 types of transactions are supported and matched on an enumeration.

### Correctness

* For the cases you are handling are you handling them correctly?
  * I did my best to research, follow the instructions carefully,
  and ask questions if I was stuck. For instance, the instructions
  do not mention whether a chargeback _has_ to follow a resolution,
  so I permitted chargebacks to follow a dispute directly.
  Since chargebacks are described as the "final state" of a dispute,
  a resolution will not modify a transaction's dispute status.
  Both deposits and withdrawals can be disputed,
  and they are handled in the same way in regard to available and held funds.
  I wasn't sure whether I need to apply signed arithmetic to based on what
  type of transaction is being disputed when calculating held funds,
  so I chose to interpret the instructions the same for both.
  I chose to mostly focus on the design of the state management and general coding practices.
* Did you test against sample data?
    * I tested against a file containing all 5 types of transactions.
      My unit tests also explore certain possible scenarios.
* Did you write unit tests for the complicated bits?
    * Unit tests are written for the transaction handlers.
* Or are you using the type system to ensure correctness?
    * I made use of the type system in instances where it was necessary,
      such as in the case of deserialization.

### Safety and Robustness

* Are you doing something dangerous? Tell us why you chose to do it this way.
    * I tried to apply the principles of ACID to my application.
        * _Atomicity_ - transactions are fully completed, or aborted if invalid.
        * _Consistency_ - new account states are defaulted to zero balances.
          All transactions connect to their corresponding client.
        * _Isolation_ - chronological synchronous processing assures that commits are isolated
        * _Durability_ - to keep the application simple I maintained state in memory.
          However, if I were to build on my solution I probably would have some sort of RDBMS
          to ensure durability.
* How are you handling errors?
    * Errors at the transaction level are simply swallowed if certain conditions are not met.
      I did not consider failing transactions as fatal errors, so I chose to ignore them.
      I/O operations however are critical to the integrity of the program so those would
      be treated as fatal errors.

### Efficiency

* Be thoughtful about how you use system resources.
    * Memory allocation was a bigger challenge than speed,
    so I did my best to have the constant time lookups that I wanted with
    as small a memory demand as I could manage.
* Can you stream values through memory as opposed to loading the entire data set upfront?
    * I used an iterator from the `csv` crate that allowed row-by-row processing
      to prevent loading all the values into memory.
      While I do maintain a hashmap per client of some transaction metadata,
      it is only for deposits and withdrawals, and only 9 bytes in size versus
      17-20 bytes.
* What if your code was bundled in a server, and these CSVs came from thousands of
  concurrent TCP streams?
    * If this was the case, I would leverage a `TcpListener` to process connections asynchronously.
      Making use of `async` threads instead of OS threads would provide better memory rationing.
      To manage client account states, synchronization could be handled by `crossbeam::channel` or
      `tokio::sync::mpsc`, so that a single consumer would interact with the client states while multiple
      workers process incoming data.

### Maintainability

* I made sure to put all state-related functionality into its own module.
  The driver program simply calls handlers depending on which type of transaction is being handled.
  These handlers can be extended to other execute checks and data-transformations. 
  