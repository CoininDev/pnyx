# Pnyx
## What is pnyx
This is a simple politic blockchain system, created to assist in direct anarchist democracies.
Inspired by Bookchin and Öcalan ideas for horizontal organisations.
This is mostly an experiment than a professional project, but I do accept and encourage contribuitions to the repository, and feedback to this project.

## Things it has
- Services system
- Cryptography
- Communication between nodes
- Storage system
- Consensus management
- Basic replaceable interface

For avoiding finantial-related terms, I decided to rename commonly known concepts in blockchain, like:
- Smart contracts -> Services
- Transactions -> Interactions

## Scopes
Services and interactions have different scopes of activity, separating in main 2 scopes:
### Constitutional
Related to universal protocols, something that is valid for the whole system, the whole "country".
### Organisational
Related to local protocols, something that is valid just in the current organisation.

Every node has the constitutional side, and one of the multiple organisational entities (organisations).

## Organisation
This is ideally pretty flexible, but I think about 4 main organisations that could exist in the pnyx ecossystem:
### Communes
The most basic human organisation, it refers to the collective of residents.
It can have local laws, local services and local open votations.
### Cooperatives
An associations of workers that produce or offer a service together, like a factory, a hospital or a school.
It can have local laws too, or decisions in finances, work or members.
### Confederations
Bookchin believes that communes can be stronger and maintaining anarchist sovereignity in confederations of multiple communes. It is not a government, but an association between multiple free communes that wants to share resources, laws and/or identity.
Here, the communes are able to coordinate themselves together.
### Militias
An organisation of militaries, with the goal of defending physically the socialist experience, this is very important and exist in Rojava, for example, for defending the kurds experience from Syrian and international attacks.
Here in pnyx, militias have their own type of organization because they have specific needs. You can have multiple militias, but they need to coordinate and work together, sharing information and forming strategically, for effectively protecting their experience.

Each organisation can have a default state of services available in the instant of creation.
As I said, this is not fixed, but it can be redefined as it becomes useful to the community.

## Individual
In pnyx, an individual is always member of at least 1 organisation(commonly the commune), but it can assotiate to other organisations.

Each individual, for access the rights to vote and interact with the system has a **keypair**, the private key is used to sign each of their interaction, and the public one is their public identity as a member of an organisation.

For making sure that an individual exists and is a valid member of an organisation, the current members needs to confirm their presence, named as "Proof-of-Personhood".