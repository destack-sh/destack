# Destack Language

Herein lies the source of truth definition for the Destack "language" - 
 that is, all the builtin constructs like Enums, Structs, Nodes, Handles with all their Properties
 and Methods and Actions and Constants and whatnot are defined here.

NOTE: None of this ships to users or the outside world directly. It is purely declaration.
We code-gen using the generate script and then use the generated code.
All the core runtime logic is in `destack-rs`. 

NOTE: The Destack "language" is a Python dialect/subset for now, but we should try to
      replace all this *.py stuff with our own *.ds once we have Descript.
	  (There is a fun bootstrapping problem there, but it is solvable.) 