# Connections

Depending on the application and its needs there are several ways to obtain a database sesssion:
- An application might use `Environment::connect` method to connect to a database and start a new user session. This is the most relevant way to get session for a single threaded application. Though, multithreaded applications might, in some cases, do the same.
- A multithreaded or a multitasking (async) application might create a session pool and then make each thread (or task) "borrow" a session from that pool for limited time. The caveat here is that those sessions are indistinguishable and thus must be "stateless".
- A blocking mode multithreaded application might create a connection pool and make each thread establish their own sessions that would use pooled connections when they need to communicate with the database. As these sessions are not shared, they can be "stateful".
