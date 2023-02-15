exports.Person = class Person {
  constructor(firstName, lastName, age) {
    Object.assign(this, { firstName, lastName, age });
  }

  fullName() {
    return Person.fullName(this.firstName, this.lastName);
  }

  static fullName(firstName, lastName) {
    return `${firstName} ${lastName}`;
  }
}
