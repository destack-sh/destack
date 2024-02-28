import { v4 } from "uuid";

export class Transaction {
  id: string;

  constructor(id: string | undefined) {
    this.id = id ?? v4();
  }

  _makeEdit() {
    
  }
}