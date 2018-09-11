import React, { Component } from 'react';
import './App.css';
import Chess from "./react-chess";

class App extends Component {

  constructor() {
    super()
    this.state = {
      lineup: Chess.getDefaultLineup(),
      legalMoves: [],
    };

    this.openConnection();
  }

  componentWillUnmount() {
    console.log("unmount");
    this.ws.close();
  }

  openConnection = () => {
    if (this.ws) {
      this.ws.close();
    }
    this.ws = new WebSocket("ws://localhost:3012");
    this.ws.addEventListener("message", this.handleMessage);
    this.ws.addEventListener("open", this.handleOpen);
    this.ws.addEventListener("close", this.handleClose);
  }


  isLegalMove = (from, to) => {
    return this.state.legalMoves.some(({from: legalFrom, to: legalTo}) => (legalFrom == from && legalTo == to));
  }

  handleOpen = (event) => {
    console.log("open");
    this.ws.send(JSON.stringify("Reset"));
  }

  handleMessage = (ev) => {
    let msg = JSON.parse(ev.data);
    console.log(msg);
    this.setState((state) => ({...state, legalMoves: msg.legal_moves, lineup: msg.lineup}));
  }

  handleClose = (ev) => {
    console.log("close", ev);
    setTimeout(this.openConnection, 3000);
  }

  handleMovePiece = (piece, from, to) => {
    this.setState((state) => {
      let lineup = this.state.lineup;
      let newLineup = [...lineup];
      if (this.isLegalMove(from, to)) {
        newLineup = this.state.lineup.map((x) => x.replace(from, to));
        this.ws.send(JSON.stringify({"Move": {from, to}}));
      } else {
        console.log("illegal move!", from, to, this.state.legalMoves);
      }
      return {...state, lineup: newLineup};
    });
  }

  render() {
    return (
      <div className="Chess">
        <Chess pieces={this.state.lineup} onMovePiece={this.handleMovePiece} test={Math.random()} />
      </div>
    );
  }
}

export default App;
